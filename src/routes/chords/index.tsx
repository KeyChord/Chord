import { useEffect, useState } from "react";
import { taurpc, type ActiveChordInfo } from "#/api/taurpc.ts";
import { Input } from "#/components/ui/input.tsx";
import { useChorderState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";
import { debug } from "@tauri-apps/plugin-log";
import getPrettyKey from "pretty-key";

export const Route = createFileRoute('/chords/')({
  component: Chords,
})

function formatKeys(keys: string[]) {
  return keys.map((key) => getPrettyKey(key)).join(" ");
}

export function Chords() {
  const [state] = useChorderState();
  const [suggestions, setSuggestions] = useState<ActiveChordInfo[]>([]);
  const hasBuffer = state.keyBuffer.length > 0;
  const inputValue = state.keyBuffer.length > 0
    ? formatKeys(state.keyBuffer)
    : state.activeChord
      ? formatKeys(state.activeChord.keys)
      : "";

  useEffect(() => {
    let cancelled = false;

    if (!hasBuffer) {
      setSuggestions([]);
      return () => {
        cancelled = true;
      };
    }

    void taurpc.listMatchingChords()
      .then((items) => {
        if (!cancelled) {
          setSuggestions(items);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setSuggestions([]);
          void debug(`Failed to load matching chords: ${error instanceof Error ? error.message : String(error)}`);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [hasBuffer, state.keyBuffer]);

  return (
    <div className="size-full bg-transparent p-3">
      <div className="flex flex-col gap-2">
      <Input
        readOnly
        value={inputValue}
        placeholder="Start typing a chord"
        className="h-11 w-full border-border/70 bg-background/85 font-mono text-base shadow-sm"
      />
        {hasBuffer && suggestions.length > 0 ? (
          <div className="overflow-hidden rounded-xl border border-border/70 bg-background/85 shadow-sm">
            {suggestions.map((suggestion) => (
              <div
                key={`${suggestion.scopeKind}:${suggestion.scope}:${suggestion.sequence}:${suggestion.name}`}
                className="grid grid-cols-[minmax(0,112px)_minmax(0,1fr)] gap-3 border-b px-3 py-2 last:border-b-0"
              >
                <div className="truncate font-mono text-xs text-foreground/80">
                  {suggestion.sequence}
                </div>
                <div className="min-w-0 truncate text-sm">
                  {suggestion.name}
                </div>
              </div>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
