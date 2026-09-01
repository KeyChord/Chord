import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  MAX_LOG_TEXT_BYTES,
  type AppLogEntry,
  type AppLogLevel,
  logMessageBytes,
  truncateLogMessage,
  useLogStream,
} from "@chord/dev.improve.chord.routes.settings._components.log-stream";
import {
  ArrowDown,
  Check,
  ChevronDown,
  ChevronUp,
  Copy,
  CornerDownLeft,
  LoaderCircle,
  Scissors,
  Search,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

const levelStyles: Record<AppLogLevel, string> = {
  trace: "text-zinc-400",
  debug: "text-zinc-400",
  info: "text-zinc-900",
  warn: "bg-amber-100 text-zinc-900",
  error: "bg-red-100 text-red-900",
};

const DIVIDER_TEXT = "-".repeat(120);
const MAX_HIGHLIGHTS_PER_MESSAGE = 20;
const MAX_LOCAL_ROWS = 256;
const MAX_SEARCH_MATCHES = 10_000;

type LocalRowKind = "command" | "divider" | "error" | "output";

interface LocalLogRow {
  afterEntry: number;
  id: number;
  kind: LocalRowKind;
  message: string;
}

type DisplayRow =
  | { entry: AppLogEntry; key: string; kind: "entry" }
  | { key: string; kind: "local"; row: LocalLogRow };

function createDisplayRows(entries: AppLogEntry[], localRows: LocalLogRow[]): DisplayRow[] {
  const rows: DisplayRow[] = [];
  const localRowsByEntry = new Map<number, LocalLogRow[]>();
  for (const row of localRows) {
    const afterEntry = Math.min(row.afterEntry, entries.length);
    const groupedRows = localRowsByEntry.get(afterEntry) ?? [];
    groupedRows.push(row);
    localRowsByEntry.set(afterEntry, groupedRows);
  }
  for (let index = 0; index <= entries.length; index += 1) {
    for (const row of localRowsByEntry.get(index) ?? []) {
      rows.push({ key: `local-${row.id}`, kind: "local", row });
    }
    const entry = entries[index];
    if (entry) {
      rows.push({ entry, key: `entry-${index}-${entry.level}`, kind: "entry" });
    }
  }
  return rows;
}

function capLocalRows(rows: LocalLogRow[]) {
  const retained: LocalLogRow[] = [];
  let retainedBytes = 0;
  for (let index = rows.length - 1; index >= 0 && retained.length < MAX_LOCAL_ROWS; index -= 1) {
    const row = rows[index];
    const message = truncateLogMessage(row.message);
    const normalized = message === row.message ? row : { ...row, message };
    const rowBytes = logMessageBytes(normalized.message);
    if (retained.length > 0 && retainedBytes + rowBytes > MAX_LOG_TEXT_BYTES) {
      break;
    }
    retained.push(normalized);
    retainedBytes += rowBytes;
  }
  retained.reverse();
  return retained;
}

function commandPrompt(command: string) {
  const trimmed = command.trim();
  return trimmed === "chord" || trimmed.startsWith("chord ")
    ? `$ ${trimmed}`
    : `$ chord ${trimmed}`;
}

function messageFromError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "Message" in error &&
    typeof error.Message === "string"
  ) {
    return error.Message;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return "Unknown error running Chord command";
  }
}

function formatCurrentTime(date: Date) {
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function findMatches(text: string, query: string) {
  if (!query) {
    return [];
  }
  const matches: number[] = [];
  const haystack = text.toLocaleLowerCase();
  const needle = query.toLocaleLowerCase();
  let start = 0;
  while (start < haystack.length && matches.length < MAX_SEARCH_MATCHES) {
    const index = haystack.indexOf(needle, start);
    if (index === -1) {
      break;
    }
    matches.push(index);
    start = index + Math.max(needle.length, 1);
  }
  return matches;
}

function highlightedMessage(message: string, query: string) {
  if (!query) {
    return message;
  }
  const parts: React.ReactNode[] = [];
  const haystack = message.toLocaleLowerCase();
  const needle = query.toLocaleLowerCase();
  let start = 0;
  let index = haystack.indexOf(needle);
  let highlighted = 0;
  while (index !== -1 && highlighted < MAX_HIGHLIGHTS_PER_MESSAGE) {
    parts.push(message.slice(start, index));
    parts.push(
      <mark key={index} className="rounded-[2px] bg-yellow-300 text-inherit">
        {message.slice(index, index + query.length)}
      </mark>,
    );
    start = index + query.length;
    index = haystack.indexOf(needle, start);
    highlighted += 1;
  }
  parts.push(message.slice(start));
  return parts;
}

export function SettingsLogPage() {
  const { clear, entries, status } = useLogStream();
  const backdropRef = useRef<HTMLPreElement>(null);
  const localRowIdRef = useRef(0);
  const scrollRef = useRef<HTMLTextAreaElement>(null);
  const [command, setCommand] = useState("");
  const [copied, setCopied] = useState(false);
  const [currentMatch, setCurrentMatch] = useState(0);
  const [currentTime, setCurrentTime] = useState(() => new Date());
  const [isRunningCommand, setIsRunningCommand] = useState(false);
  const [isFollowing, setIsFollowing] = useState(true);
  const [localRows, setLocalRows] = useState<LocalLogRow[]>([]);
  const [query, setQuery] = useState("");
  const displayRows = useMemo(() => createDisplayRows(entries, localRows), [entries, localRows]);
  const logText = useMemo(
    () =>
      displayRows
        .map((row) => (row.kind === "local" ? row.row.message : row.entry.message))
        .join("\n"),
    [displayRows],
  );
  const matches = useMemo(() => findMatches(logText, query), [logText, query]);

  function syncBackdrop(scrollElement: HTMLTextAreaElement) {
    if (backdropRef.current) {
      backdropRef.current.style.transform = `translateY(${-scrollElement.scrollTop}px)`;
    }
  }

  useEffect(() => {
    const interval = window.setInterval(() => setCurrentTime(new Date()), 1_000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    setCurrentMatch(0);
  }, [query]);

  useEffect(() => {
    if (!isFollowing) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      const scrollElement = scrollRef.current;
      if (scrollElement) {
        scrollElement.scrollTop = scrollElement.scrollHeight;
        syncBackdrop(scrollElement);
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [isFollowing, logText]);

  function handleScroll() {
    const scrollElement = scrollRef.current;
    if (!scrollElement) {
      return;
    }
    syncBackdrop(scrollElement);
    const distanceFromBottom =
      scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight;
    setIsFollowing(distanceFromBottom < 48);
  }

  function showMatch(direction: 1 | -1) {
    if (matches.length === 0) {
      return;
    }
    const nextMatch = (currentMatch + direction + matches.length) % matches.length;
    setCurrentMatch(nextMatch);
    setIsFollowing(false);
    const textarea = scrollRef.current;
    if (textarea) {
      const matchStart = matches[nextMatch];
      textarea.focus();
      textarea.setSelectionRange(matchStart, matchStart + query.length);
    }
  }

  function jumpToLatest() {
    setIsFollowing(true);
    const scrollElement = scrollRef.current;
    if (scrollElement) {
      scrollElement.scrollTo({ top: scrollElement.scrollHeight, behavior: "smooth" });
    }
  }

  async function copyLogs() {
    await navigator.clipboard.writeText(logText);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1_500);
  }

  function clearLogs() {
    clear();
    setLocalRows([]);
    setIsFollowing(true);
  }

  function appendLocalRows(
    afterEntry: number,
    rows: { kind: LocalRowKind; message: string }[],
  ) {
    setLocalRows((previous) =>
      capLocalRows([
        ...previous,
        ...rows.map((row) => {
          localRowIdRef.current += 1;
          return {
            ...row,
            message: truncateLogMessage(row.message),
            afterEntry,
            id: localRowIdRef.current,
          };
        }),
      ]),
    );
  }

  function addDivider() {
    appendLocalRows(entries.length, [{ kind: "divider", message: DIVIDER_TEXT }]);
    setIsFollowing(true);
  }

  async function runCommand(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const submittedCommand = command.trim();
    if (!submittedCommand || isRunningCommand) {
      return;
    }

    const afterEntry = entries.length;
    setCommand("");
    setIsRunningCommand(true);
    setIsFollowing(true);
    appendLocalRows(afterEntry, [{ kind: "command", message: commandPrompt(submittedCommand) }]);

    try {
      const output = await taurpc.runChordCommand(submittedCommand);
      const rows: { kind: LocalRowKind; message: string }[] = [];
      const stdout = output.stdout.trimEnd();
      const stderr = output.stderr.trimEnd();
      if (stdout) {
        rows.push({ kind: "output", message: stdout });
      }
      if (stderr) {
        rows.push({ kind: "error", message: stderr });
      }
      if (!stdout && !stderr && output.exitCode !== 0) {
        rows.push({
          kind: "error",
          message: `Chord command exited with code ${output.exitCode ?? "unknown"}.`,
        });
      }
      appendLocalRows(afterEntry, rows);
    } catch (error) {
      appendLocalRows(afterEntry, [{ kind: "error", message: messageFromError(error) }]);
    } finally {
      setIsRunningCommand(false);
      setIsFollowing(true);
    }
  }

  const statusLabel =
    status === "live" ? "Live" : status === "connecting" ? "Connecting" : "Stream unavailable";

  return (
    <section className="flex h-full min-h-0 flex-col gap-3">
      <h2 className="shrink-0 text-[15px] font-semibold tracking-[-0.01em]">Application Log</h2>

      <div className="flex shrink-0 items-center justify-between gap-3">
        <div className="flex items-center gap-1.5">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={logText.length === 0}
            onClick={() => void copyLogs()}
          >
            {copied ? <Check /> : <Copy />}
            {copied ? "Copied" : "Copy to pasteboard"}
          </Button>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={logText.length === 0 || isRunningCommand}
            onClick={clearLogs}
          >
            <Trash2 />
            Clear
          </Button>
        </div>
      </div>

      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden border bg-white shadow-sm">
        <div className="flex h-10 shrink-0 items-center gap-2 border-b bg-muted/20 px-2">
          <div className="flex min-w-0 flex-1 items-center gap-1.5 rounded-md border bg-white px-2 focus-within:border-blue-400 focus-within:ring-2 focus-within:ring-blue-200">
            <Search className="size-3.5 shrink-0 text-muted-foreground" />
            <input
              type="search"
              className="h-7 min-w-0 flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground"
              value={query}
              placeholder="Find"
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  showMatch(event.shiftKey ? -1 : 1);
                } else if (event.key === "Escape") {
                  setQuery("");
                }
              }}
            />
            {query ? (
              <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">
                {matches.length === 0 ? "No matches" : `${currentMatch + 1} of ${matches.length}`}
              </span>
            ) : null}
          </div>
          <div className="flex shrink-0 items-center overflow-hidden rounded-md border bg-background">
            <button
              type="button"
              className="flex size-7 items-center justify-center border-r text-muted-foreground hover:bg-muted disabled:opacity-35"
              disabled={matches.length === 0}
              onClick={() => showMatch(-1)}
              aria-label="Previous match"
            >
              <ChevronUp className="size-3.5" />
            </button>
            <button
              type="button"
              className="flex size-7 items-center justify-center text-muted-foreground hover:bg-muted disabled:opacity-35"
              disabled={matches.length === 0}
              onClick={() => showMatch(1)}
              aria-label="Next match"
            >
              <ChevronDown className="size-3.5" />
            </button>
          </div>
          <Button type="button" variant="secondary" size="sm" onClick={() => setQuery("")}>
            Done
          </Button>
        </div>

        <div className="relative min-h-0 flex-1 overflow-hidden bg-white">
          <pre
            ref={backdropRef}
            aria-hidden="true"
            className="pointer-events-none absolute left-0 top-0 m-0 w-full whitespace-pre-wrap break-words px-3.5 py-3 text-[11.5px] leading-[1.55]"
            style={{
              fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
            }}
          >
            {displayRows.map((row) =>
              row.kind === "local" && row.row.kind === "divider" ? (
                <span
                  key={row.key}
                  className="block min-h-[1lh] overflow-hidden bg-zinc-900 text-white"
                >
                  {row.row.message}
                </span>
              ) : row.kind === "local" ? (
                <span
                  key={row.key}
                  className={
                    row.row.kind === "error"
                      ? "block min-h-[1lh] bg-red-100 text-red-900"
                      : row.row.kind === "command"
                        ? "block min-h-[1lh] bg-zinc-100 font-semibold text-zinc-700"
                        : "block min-h-[1lh] text-zinc-900"
                  }
                >
                  {highlightedMessage(row.row.message, query)}
                </span>
              ) : (
                <span key={row.key} className={`block min-h-[1lh] ${levelStyles[row.entry.level]}`}>
                  {highlightedMessage(row.entry.message, query)}
                </span>
              ),
            )}
          </pre>
          <textarea
            ref={scrollRef}
            className="absolute inset-0 z-10 size-full resize-none overflow-auto whitespace-pre-wrap break-words border-0 bg-transparent px-3.5 py-3 text-[11.5px] leading-[1.55] text-transparent caret-zinc-900 outline-none placeholder:text-zinc-400 selection:bg-blue-200/60 selection:text-transparent"
            style={{
              fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
            }}
            value={logText}
            placeholder="No log entries yet. New app activity will appear here."
            readOnly
            spellCheck={false}
            wrap="soft"
            onScroll={handleScroll}
            aria-label="Application log entries"
          />
        </div>

        <form
          className="flex shrink-0 items-center gap-2 border-t bg-zinc-50 px-2.5 py-2"
          onSubmit={(event) => void runCommand(event)}
        >
          <span
            className="shrink-0 select-none text-xs font-semibold text-zinc-500"
            style={{
              fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
            }}
          >
            chord
          </span>
          <input
            type="text"
            className="h-8 min-w-0 flex-1 rounded-md border bg-white px-2.5 text-xs outline-none placeholder:text-muted-foreground focus:border-blue-400 focus:ring-2 focus:ring-blue-200"
            value={command}
            disabled={isRunningCommand}
            placeholder="help"
            autoCapitalize="off"
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => setCommand(event.target.value)}
            aria-label="Chord command"
          />
          <Button type="submit" size="sm" disabled={!command.trim() || isRunningCommand}>
            {isRunningCommand ? <LoaderCircle className="animate-spin" /> : <CornerDownLeft />}
            {isRunningCommand ? "Running" : "Run"}
          </Button>
        </form>

        {!isFollowing && entries.length > 0 ? (
          <Button
            type="button"
            variant="secondary"
            size="sm"
            className="absolute bottom-14 left-1/2 -translate-x-1/2 shadow-lg"
            onClick={jumpToLatest}
          >
            <ArrowDown />
            Latest
          </Button>
        ) : null}
      </div>

      <div className="flex shrink-0 items-center justify-between gap-4 text-[11px] text-muted-foreground">
        <div className="flex items-center gap-3">
          <span className="text-xs text-foreground">
            Current time: [{formatCurrentTime(currentTime)}]
          </span>
          <Button type="button" variant="secondary" size="sm" onClick={addDivider}>
            <Scissors />
            Add divider
          </Button>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span
              className={
                status === "live"
                  ? "size-1.5 rounded-full bg-emerald-500"
                  : "size-1.5 rounded-full bg-amber-500"
              }
            />
            <span>{statusLabel}</span>
          </div>
          <span className="tabular-nums">
            {entries.length.toLocaleString()} {entries.length === 1 ? "entry" : "entries"}
          </span>
        </div>
      </div>
    </section>
  );
}
