import { useEffect, useState } from "react";
import { taurpc, type ActiveChordInfo } from "#/api/taurpc.ts";
import { Kbd } from "#/components/ui/kbd.tsx";
import { useChorderState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { debug } from "@tauri-apps/plugin-log";
import getPrettyKey from "pretty-key";
import { cn } from "#/utils/style.ts";

export const Route = createFileRoute('/chords/')({
  component: Chords,
})

function formatKeys(keys: string[]) {
  return keys.map((key) => getPrettyKey(key)).join(" ");
}

const LETTER_TOKENS = Array.from({ length: 26 }, (_, index) =>
  String.fromCharCode("A".charCodeAt(0) + index),
);
const MAX_KEY_SIZE = 32;

function compareSymbolTokens(left: string, right: string) {
  return left.localeCompare(right);
}

function isSingleCharacterToken(token: string) {
  return token.length === 1;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function compareSuggestionPriority(left: ActiveChordInfo, right: ActiveChordInfo) {
  const leftDescriptionRank = left.isDescription ? 0 : 1;
  const rightDescriptionRank = right.isDescription ? 0 : 1;
  const leftScopeRank = left.scopeKind === "app" ? 0 : 1;
  const rightScopeRank = right.scopeKind === "app" ? 0 : 1;

  return leftDescriptionRank - rightDescriptionRank
    || leftScopeRank - rightScopeRank
    || left.sequence.localeCompare(right.sequence)
    || left.name.localeCompare(right.name);
}

function resolveTokenDescription(
  suggestions: ActiveChordInfo[],
  prefixTokens: string[],
  token: string,
) {
  const sequence = [...prefixTokens, token].join(" ");
  let bestMatch: ActiveChordInfo | undefined;

  for (const suggestion of suggestions) {
    if (suggestion.sequence !== sequence) {
      continue;
    }

    if (!bestMatch || compareSuggestionPriority(suggestion, bestMatch) < 0) {
      bestMatch = suggestion;
    }
  }

  if (!bestMatch) {
    return "";
  }

  return bestMatch.description ?? (!bestMatch.isDescription ? bestMatch.name : "");
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
      <div style={{ fontSize: `${descriptionFontSize}px` }}>
        {description}
      </div>
    </div>
  );
}

export function Chords() {
  const [state] = useChorderState();
  const [allSuggestions, setAllSuggestions] = useState<ActiveChordInfo[]>([]);
  const [suggestions, setSuggestions] = useState<ActiveChordInfo[]>([]);
  const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight);
  const [surfaceVersion, setSurfaceVersion] = useState(0);
  const currentPrefixLength = state.keyBuffer.length;

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
    let cancelled = false;

    void taurpc.listMatchingChords()
      .then((items) => {
        if (!cancelled) {
          if (currentPrefixLength === 0) {
            setAllSuggestions(items);
          }
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
  }, [currentPrefixLength, state.keyBuffer]);

  useEffect(() => {
    let firstFrame = 0;
    let secondFrame = 0;
    const unlistenPromise = listen<boolean>("chorder-visibility-changed", (event) => {
      if (!event.payload) {
        return;
      }

      cancelAnimationFrame(firstFrame);
      cancelAnimationFrame(secondFrame);
      firstFrame = requestAnimationFrame(() => {
        secondFrame = requestAnimationFrame(() => {
          setSurfaceVersion((version) => version + 1);
        });
      });
    });

    return () => {
      cancelAnimationFrame(firstFrame);
      cancelAnimationFrame(secondFrame);
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, []);

  const sequenceSource = allSuggestions.length > 0 ? allSuggestions : suggestions;
  const allSequences = sequenceSource.map((suggestion) => suggestion.sequence.split(" "));
  const normalizedBufferTokens = state.keyBuffer.map((key) => {
    const pretty = getPrettyKey(key);
    return pretty.length === 1 ? pretty.toUpperCase() : pretty;
  });

  const allSymbolTokens = [...new Set(
    allSequences
      .flatMap((sequence) => sequence)
      .filter((token) => isSingleCharacterToken(token) && !/^[A-Z0-9]$/.test(token)),
  )].sort(compareSymbolTokens);
  const maxVisibleRows = Math.max(
    1,
    ...Array.from({ length: Math.max(1, currentPrefixLength + 1) }, (_, columnIndex) => {
      const prefixTokens = normalizedBufferTokens.slice(0, columnIndex);
      const activeTokens = new Set(
        allSequences
          .filter((sequence) =>
            prefixTokens.every((token, tokenIndex) => sequence[tokenIndex] === token)
          )
          .map((sequence) => sequence[columnIndex])
          .filter((token): token is string => Boolean(token)),
      );
      const letterCount = LETTER_TOKENS.filter((token) => activeTokens.has(token)).length;
      const symbolCount = allSymbolTokens.filter((token) => activeTokens.has(token)).length;
      return letterCount + symbolCount + (symbolCount > 0 ? 2 : 0);
    }),
  );
  const availableHeight = Math.max(viewportHeight - 96, 240);
  const idealKeySize = availableHeight / (maxVisibleRows + Math.max(maxVisibleRows - 1, 0) * 0.18);
  const keySize = clamp(Math.floor(idealKeySize), 22, MAX_KEY_SIZE);
  const rowGap = clamp(
    Math.floor((availableHeight - keySize * maxVisibleRows) / Math.max(maxVisibleRows - 1, 1)),
    4,
    10,
  );
  const descriptionFontSize = clamp(Math.round(keySize * 0.42), 11, 16);
  const keyColumns = Array.from({ length: Math.max(1, currentPrefixLength + 1) }, (_, columnIndex) => {
    const prefixTokens = normalizedBufferTokens.slice(0, columnIndex);
    const activeTokens = new Set(
      allSequences
        .filter((sequence) =>
          prefixTokens.every((token, tokenIndex) => sequence[tokenIndex] === token)
        )
        .map((sequence) => sequence[columnIndex])
        .filter((token): token is string => Boolean(token)),
    );

    return {
      id: `column-${columnIndex}`,
      prefixTokens,
      activeTokens,
      selectedToken: normalizedBufferTokens[columnIndex],
      hasSelection: Boolean(normalizedBufferTokens[columnIndex]),
    };
  });

  return (
    <div className="relative size-full bg-transparent">
      <div className="absolute left-0 top-1/2 -translate-y-1/2">
        <div
          key={surfaceVersion}
          style={{
            backdropFilter: "blur(52px) saturate(1.8)",
            WebkitBackdropFilter: "blur(52px) saturate(1.8)",
            transform: "translateZ(0)",
            willChange: "backdrop-filter, -webkit-backdrop-filter",
          }}
          className={cn(
            "relative isolate overflow-hidden rounded-r-[2rem] rounded-l-none border border-l-0 px-5 py-5 pl-7 transform-gpu",
            "border-white/28 bg-white/30 shadow-[18px_20px_60px_rgba(15,23,42,0.18),inset_0_1px_0_rgba(255,255,255,0.42)]",
            "dark:border-white/10 dark:bg-zinc-950/34 dark:shadow-[18px_20px_60px_rgba(0,0,0,0.34),inset_0_1px_0_rgba(255,255,255,0.1)]",
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
                      description={resolveTokenDescription(sequenceSource, column.prefixTokens, token)}
                      isSelected={column.selectedToken === token}
                      isDimmed={column.hasSelection && column.selectedToken !== token}
                      keySize={keySize}
                      descriptionFontSize={descriptionFontSize}
                    />
                  ))}

                  {allSymbolTokens.some((token) => column.activeTokens.has(token)) ? (
                    <div
                      className="mt-2 flex flex-col items-start"
                      style={{ gap: `${rowGap}px` }}
                    >
                      {allSymbolTokens.filter((token) => column.activeTokens.has(token)).map((token) => (
                        <ChordKeyRow
                          key={`${column.id}-${token}`}
                          token={token}
                          description={resolveTokenDescription(sequenceSource, column.prefixTokens, token)}
                          isSelected={column.selectedToken === token}
                          isDimmed={column.hasSelection && column.selectedToken !== token}
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
