import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Kbd } from "#/components/ui/kbd.tsx";
import { useChorderState, useChordRegistryState, useFrontmostState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";
import { emit, listen } from "@tauri-apps/api/event";
import getPrettyKey from "pretty-key";
import { cn } from "#/utils/style.ts";

export const Route = createFileRoute("/chords/")({
  component: Chords,
});

const LETTER_TOKENS = Array.from({ length: 26 }, (_, index) =>
  String.fromCharCode("A".charCodeAt(0) + index),
);
const MAX_KEY_SIZE = 32;
const NATIVE_SURFACE_RADIUS = 32;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function normalizePrettyKey(token: string) {
  if (token === "_") {
    return "-";
  }

  return token;
}

function ChordKeyRow({
  token,
  description = "",
  isSelected = false,
  isDimmed = false,
  keySize,
  descriptionFontSize,
}: {
  token: string;
  description?: string;
  isSelected?: boolean;
  isDimmed?: boolean;
  keySize: number;
  descriptionFontSize: number;
}) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 transition-all",
        isDimmed ? "opacity-35" : "opacity-100",
        "text-foreground/95",
      )}
    >
      <Kbd
        style={{
          height: `${keySize}px`,
          minWidth: `${keySize}px`,
          fontSize: `${Math.max(12, Math.round(keySize * 0.48))}px`,
        }}
        className={cn(
          "rounded-md border px-0 font-mono shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgba(0,0,0,0.18)]",
          isSelected
            ? "border-emerald-400/90 bg-emerald-100 text-emerald-950 shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(52,211,153,0.35),0_4px_10px_rgba(16,185,129,0.25)]"
            : "border-border/80 bg-background/95 text-foreground",
        )}
      >
        {token}
      </Kbd>
      <div style={{ fontSize: `${descriptionFontSize}px` }}>{description}</div>
    </div>
  );
}

export function Chords() {
  const state = useChorderState();
  const { chords } = useChordRegistryState();
  const { frontmostAppBundleId } = useFrontmostState()
  const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight);
  const [surfaceVersion, setSurfaceVersion] = useState(0);
  const [isPreparingSurface, setIsPreparingSurface] = useState(false);
  const surfaceRef = useRef<HTMLDivElement>(null);
  const currentPrefixLength = state.keyBuffer.length;

  const emitSurfaceRect = () => {
    const surface = surfaceRef.current;
    if (!surface) {
      return;
    }

    const rect = surface.getBoundingClientRect();
    void emit("chorder-surface-rect", {
      x: rect.left,
      y: window.innerHeight - rect.bottom,
      width: rect.width,
      height: rect.height,
      radius: NATIVE_SURFACE_RADIUS,
    });
  };

  useEffect(() => {
    const handleResize = () => {
      setViewportHeight(window.innerHeight);
    };

    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  useEffect(() => {
    const unlistenPromise = listen("chorder-will-show", () => {
      setIsPreparingSurface(true);
      setSurfaceVersion((version) => version + 1);
    });

    return () => {
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, []);

  useEffect(() => {
    void emit("chorder-window-ready");
  }, []);

  useLayoutEffect(() => {
    if (!isPreparingSurface) {
      return;
    }

    emitSurfaceRect();
    void emit("chorder-surface-ready");
    setIsPreparingSurface(false);
  }, [isPreparingSurface, surfaceVersion]);

  useEffect(() => {
    const surface = surfaceRef.current;
    if (!surface) {
      return;
    }

    const observer = new ResizeObserver(() => {
      emitSurfaceRect();
    });
    observer.observe(surface);

    return () => {
      observer.disconnect();
    };
  }, [surfaceVersion]);

  const normalizedBufferTokens = state.keyBuffer.map((key) => {
    const pretty = normalizePrettyKey(getPrettyKey(key));
    return pretty.length === 1 ? pretty.toUpperCase() : pretty;
  });

  const maxVisibleRows = 20;
  const availableHeight = Math.max(viewportHeight - 96, 240);
  const idealKeySize = availableHeight / (maxVisibleRows + Math.max(maxVisibleRows - 1, 0) * 0.18);
  const keySize = clamp(Math.floor(idealKeySize), 22, MAX_KEY_SIZE);
  const rowGap = clamp(
    Math.floor((availableHeight - keySize * maxVisibleRows) / Math.max(maxVisibleRows - 1, 1)),
    4,
    10,
  );
  const descriptionFontSize = clamp(Math.round(keySize * 0.42), 11, 16);
  const keyColumns = Array.from(
    { length: Math.max(1, currentPrefixLength + 1) },
    (_, columnIndex) => {
      const prefixTokens = normalizedBufferTokens.slice(0, columnIndex);
      const activeTokens = new Set(
        chords
          .filter((chord) =>
            prefixTokens.every((token, tokenIndex) => chord.keys[tokenIndex] === token),
          )
          .map((chord) => chord.keys[columnIndex])
          .filter((token): token is string => Boolean(token)),
      );

      return {
        id: `column-${columnIndex}`,
        prefixTokens,
        activeTokens,
        selectedToken: normalizedBufferTokens[columnIndex],
        hasSelection: Boolean(normalizedBufferTokens[columnIndex]),
      };
    },
  );

  useLayoutEffect(() => {
    emitSurfaceRect();
  }, [currentPrefixLength, keyColumns.length, keySize, rowGap, descriptionFontSize]);

  return (
    <div className="relative size-full bg-transparent">
      <div className="absolute left-0 top-1/2 -translate-y-1/2">
        <div
          key={surfaceVersion}
          ref={surfaceRef}
          className={cn(
            "relative isolate overflow-hidden rounded-r-[2rem] rounded-l-none border border-l-0 px-5 py-5 pl-7",
            "border-white/30 bg-white/22 shadow-[18px_20px_60px_rgba(15,23,42,0.18),inset_0_1px_0_rgba(255,255,255,0.42)]",
            "dark:border-white/10 dark:bg-zinc-950/24 dark:shadow-[18px_20px_60px_rgba(0,0,0,0.34),inset_0_1px_0_rgba(255,255,255,0.1)]",
          )}
        >
          <div className="relative flex items-start">
            <div className="flex items-start gap-6">
              {keyColumns.map((column) => (
                <div
                  key={column.id}
                  className="flex flex-col items-start justify-center"
                  style={{ gap: `${rowGap}px` }}
                >
                  {LETTER_TOKENS.filter((token) => column.activeTokens.has(token)).map((token) => (
                    <ChordKeyRow
                      key={`${column.id}-${token}`}
                      token={token}
                      description='todo'
                      isSelected={column.selectedToken === token}
                      isDimmed={column.hasSelection && column.selectedToken !== token}
                      keySize={keySize}
                      descriptionFontSize={descriptionFontSize}
                    />
                  ))}

                  {chords.some((chord) => column.activeTokens.has(chord.keys[0])) ? (
                    <div className="flex flex-col items-start" style={{ gap: `${rowGap}px` }}>
                      {chords
                        .filter((chord) => column.activeTokens.has(chord.keys[0]))
                        .map((chord) => (
                          <ChordKeyRow
                            key={`${column.id}-${chord.keys[0]}`}
                            token={chord.keys[0]}
                            description='todo'
                            isSelected={column.selectedToken === chord.keys[0]}
                            isDimmed={column.hasSelection && column.selectedToken !== chord.keys[0]}
                            keySize={keySize}
                            descriptionFontSize={descriptionFontSize}
                          />
                        ))}
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
