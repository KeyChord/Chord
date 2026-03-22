import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Kbd, KbdGroup } from "#/components/ui/kbd.tsx";
import { cn } from "#/utils/style.ts";
import { useChorderState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute('/chords/')({
  component: Chords,
})

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
    <KbdGroup className="flex flex-wrap items-center justify-center gap-3">
      {keys.map((key, index) => (
        <Kbd
          key={`${key}-${index}`}
          className={cn(
            "h-12 min-w-14 rounded-2xl border px-4 py-3 text-center text-lg font-semibold tracking-[0.12em] shadow-[0_16px_40px_rgba(15,23,42,0.3)]",
            dimmed
              ? "border-white/10 bg-white/8 text-white/55"
              : "border-white/20 bg-white/14 text-white",
          )}
        >
          {key}
        </Kbd>
      ))}
    </KbdGroup>
  );
}

export function Chords() {
  const [state] = useChorderState();

  if (state === null) {
    return null;
  }

  const hasBuffer = state.keyBuffer.length > 0;
  const hasActive = state.activeChord !== undefined;
  const displayKeys = hasBuffer ? state.keyBuffer : state.activeChord?.keys ?? [];
  const helperLabel = hasBuffer
    ? "Buffer"
    : hasActive
      ? "Active chord"
      : "Ready";

  return (
    <div className="flex size-full items-center justify-center overflow-hidden bg-[radial-gradient(circle_at_top,rgba(34,211,238,0.14),transparent_38%),#08111f] px-4 py-6 text-white">
      <Card className="w-full max-w-xl gap-0 rounded-[28px] border-cyan-200/15 bg-[linear-gradient(180deg,rgba(30,41,59,0.96),rgba(8,15,27,0.99))] py-0 text-white ring-cyan-200/10 shadow-[0_24px_80px_rgba(2,12,27,0.7)]">
        <CardHeader className="border-b border-white/10 px-8 py-6">
          <div className="flex items-center justify-between gap-3">
            <div className="space-y-2">
              <Badge
                variant="outline"
                className="border-cyan-200/20 bg-cyan-300/10 text-cyan-100"
              >
                Chord Mode
              </Badge>
              <div>
                <CardTitle className="text-lg font-semibold tracking-[0.08em] text-white">
                  {hasActive ? state.activeChord?.name ?? "Active chord" : "Chord input"}
                </CardTitle>
                <CardDescription className="text-white/55">
                  {hasActive
                    ? "The current chord remains visible until the next input starts."
                    : "Start typing a chord."}
                </CardDescription>
              </div>
            </div>
            <Badge variant="secondary" className="bg-white/10 text-white/70">
              {helperLabel}
            </Badge>
          </div>
        </CardHeader>

        <CardContent className="px-8 py-6">
          {displayKeys.length > 0 ? (
            <KeyCapsRow keys={displayKeys} dimmed={!hasBuffer} />
          ) : (
            <div className="text-center text-lg font-medium tracking-[0.08em] text-white/55">
              Start typing a chord
            </div>
          )}
        </CardContent>

        {hasActive ? (
          <CardContent className="border-t border-white/10 px-8 pb-6 pt-4 text-sm text-white/60">
            {state.activeChord?.shortcut
              ? "Shortcut-backed chord is armed."
              : "Manual chord mode is active."}
          </CardContent>
        ) : (
          null
        )}
      </Card>
    </div>
  );
}
