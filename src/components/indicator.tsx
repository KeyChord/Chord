import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type ChorderIndicatorState = {
  visible: boolean;
  bufferKeys: string[];
  activeKeys: string[];
  shiftPressed: boolean;
};

const INITIAL_STATE: ChorderIndicatorState = {
  visible: false,
  bufferKeys: [],
  activeKeys: [],
  shiftPressed: false,
};

function KeyCapsRow({
  keys,
  dimmed = false,
}: {
  keys: string[];
  dimmed?: boolean;
}) {
  if (keys.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-wrap items-center justify-center gap-3">
      {keys.map((key, index) => (
        <div
          key={`${key}-${index}`}
          className={[
            "min-w-14 rounded-2xl border px-4 py-3 text-center text-lg font-semibold tracking-[0.12em] shadow-[0_16px_40px_rgba(15,23,42,0.3)]",
            dimmed
              ? "border-white/10 bg-white/8 text-white/55"
              : "border-white/20 bg-white/14 text-white",
          ].join(" ")}
        >
          {key}
        </div>
      ))}
    </div>
  );
}

export function ChordIndicatorWindow() {
  const [state, setState] = useState<ChorderIndicatorState>(INITIAL_STATE);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      const unlisten = await listen<ChorderIndicatorState>(
        "chorder-indicator-state-changed",
        (event) => {
          if (!cancelled) {
            setState(event.payload);
          }
        },
      );

      if (cancelled) {
        unlisten();
      }

      return unlisten;
    };

    const cleanupPromise = setup();

    return () => {
      cancelled = true;
      void cleanupPromise.then((cleanup) => cleanup?.());
    };
  }, []);

  const hasBuffer = state.bufferKeys.length > 0;
  const hasActive = state.activeKeys.length > 0;
  const helperLabel = hasBuffer
    ? state.shiftPressed
      ? "Shift pressed"
      : "Buffer"
    : hasActive
      ? "Active chord"
      : "Ready";

  return (
    <div className="flex size-full items-center justify-center overflow-hidden bg-[#08111f] text-white">
      <div className="flex w-full flex-col gap-5 rounded-[28px] border border-cyan-200/15 bg-[linear-gradient(180deg,rgba(30,41,59,0.96),rgba(8,15,27,0.99))] px-8 py-6 shadow-[0_24px_80px_rgba(2,12,27,0.7)]">
        <div className="flex items-center justify-between">
          <div className="text-sm font-semibold uppercase tracking-[0.32em] text-cyan-200/80">
            Chord Mode
          </div>
          <div className="text-xs font-medium uppercase tracking-[0.24em] text-white/45">
            {helperLabel}
          </div>
        </div>

        {hasBuffer ? (
          <KeyCapsRow keys={state.bufferKeys} />
        ) : hasActive ? (
          <KeyCapsRow keys={state.activeKeys} dimmed />
        ) : (
          <div className="text-center text-lg font-medium tracking-[0.08em] text-white/55">
            Start typing a chord
          </div>
        )}
      </div>
    </div>
  );
}
