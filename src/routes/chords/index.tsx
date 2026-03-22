import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Kbd, KbdGroup } from "#/components/ui/kbd.tsx";
import { useChorderState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";
import { debug } from "@tauri-apps/plugin-log";
import getPrettyKey from "pretty-key";

export const Route = createFileRoute('/chords/')({
  component: Chords,
})

function KeyCapsRow({
  keys,
}: {
  keys: string[];
}) {
  if (keys.length === 0) {
    return null;
  }

  return (
    <KbdGroup className="flex flex-wrap items-center justify-center gap-2">
      {keys.map((key, index) => <Kbd key={`${key}-${index}`}>{key}</Kbd>)}
    </KbdGroup>
  );
}

export function Chords() {
  const [state] = useChorderState();
  const hasBuffer = state.keyBuffer.length > 0;
  const hasActive = state.activeChord !== undefined;
  const displayKeys = hasBuffer ? state.keyBuffer.map(key => getPrettyKey(key)) : state.activeChord?.keys.map(key => getPrettyKey(key)) ?? [];
  const helperLabel = hasBuffer
    ? "Buffer"
    : hasActive
      ? "Active chord"
      : "Ready";
  const statusText = hasActive
    ? state.activeChord?.shortcut
      ? "Shortcut-backed chord is armed."
      : "Manual chord mode is active."
    : "Start typing a chord.";

  return (
    <div className="size-full bg-transparent">
      <Card className="size-full justify-between gap-0 py-0">
        <CardHeader className="border-b">
          <div className="flex items-start justify-between gap-3">
            <div className="space-y-1">
              <Badge variant="outline">Chord Mode</Badge>
              <div>
                <CardTitle>
                  {hasActive ? state.activeChord?.name ?? "Active chord" : "Chord input"}
                </CardTitle>
                <CardDescription>
                  {hasActive
                    ? "The current chord remains visible until the next input starts."
                    : "Ready for input."}
                </CardDescription>
              </div>
            </div>
            <Badge variant={hasBuffer ? "default" : "secondary"}>{helperLabel}</Badge>
          </div>
        </CardHeader>

        <CardContent className="flex flex-1 items-center justify-center">
          {displayKeys.length > 0 ? (
            <KeyCapsRow keys={displayKeys} />
          ) : (
            <div className="text-center text-sm text-muted-foreground">
              Start typing a chord
            </div>
          )}
        </CardContent>

        <CardFooter className="justify-center text-muted-foreground">
          {statusText}
        </CardFooter>
      </Card>
    </div>
  );
}
