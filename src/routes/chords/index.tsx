import { useEffect, useState } from "react";
import { listMatchingChords, type ActiveChordInfo } from "#/api/settings.ts";
import { Badge } from "#/components/ui/badge.tsx";
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
  const [loadingSuggestions, setLoadingSuggestions] = useState(false);

  const hasBuffer = state.keyBuffer.length > 0;
  const hasActive = state.activeChord !== undefined;
  const helperLabel = hasBuffer
    ? "Buffer"
    : hasActive
      ? "Active chord"
      : "Ready";
  const statusText = hasBuffer
    ? "Matching chords update from the Rust runtime as keys enter the buffer."
    : hasActive
      ? "The last active chord stays visible until the next input starts."
      : "Start typing to see matching chords.";
  const inputValue = hasBuffer
    ? formatKeys(state.keyBuffer)
    : state.activeChord
      ? formatKeys(state.activeChord.keys)
      : "";

  useEffect(() => {
    let cancelled = false;

    if (!hasBuffer) {
      setSuggestions([]);
      setLoadingSuggestions(false);
      return () => {
        cancelled = true;
      };
    }

    setLoadingSuggestions(true);

    void listMatchingChords()
      .then((items) => {
        if (cancelled) {
          return;
        }

        setSuggestions(items);
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setSuggestions([]);
        void debug(`Failed to load matching chords: ${error instanceof Error ? error.message : String(error)}`);
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingSuggestions(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [hasBuffer, state.keyBuffer]);

  return (
    <div className="size-full bg-transparent p-3">
      <div className="mx-auto flex h-full max-w-3xl flex-col gap-3">
        <div className="flex items-start justify-between gap-3">
          <div className="space-y-1">
            <div className="text-sm font-medium">Chord input</div>
            <div className="text-xs text-muted-foreground">
              {statusText}
            </div>
          </div>
          <Badge variant={hasBuffer ? "default" : "secondary"}>{helperLabel}</Badge>
        </div>

        <div className="overflow-hidden rounded-xl border bg-background/85 shadow-sm">
          <div className="border-b p-3">
            <Input
              readOnly
              value={inputValue}
              placeholder="Start typing a chord"
              className="h-11 border-border/70 bg-background font-mono text-base"
            />
          </div>

          <div className="max-h-full overflow-y-auto p-2">
            {loadingSuggestions ? (
              <div className="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                Finding matching chords...
              </div>
            ) : hasBuffer ? (
              suggestions.length > 0 ? (
                <div className="space-y-2">
                  {suggestions.map((suggestion) => (
                    <div
                      key={`${suggestion.scopeKind}:${suggestion.scope}:${suggestion.sequence}:${suggestion.name}`}
                      className="grid grid-cols-[minmax(0,120px)_minmax(0,1fr)] gap-3 rounded-lg border bg-background/80 px-3 py-2"
                    >
                      <div className="truncate font-mono text-xs text-foreground/85">
                        {suggestion.sequence}
                      </div>
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="truncate text-sm font-medium">{suggestion.name}</span>
                          <Badge variant={suggestion.scopeKind === "global" ? "secondary" : "outline"}>
                            {suggestion.scopeKind === "global" ? "Global" : suggestion.scope}
                          </Badge>
                        </div>
                        <div className="truncate text-xs text-muted-foreground">
                          {suggestion.action}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                  No chords match the current buffer.
                </div>
              )
            ) : (
              <div className="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                Start typing to populate chord suggestions.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
