import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { attachLogger, LogLevel } from "@tauri-apps/plugin-log";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

const MAX_LOG_ENTRIES = 2_000;
export const MAX_LOG_MESSAGE_BYTES = 64 * 1024;
export const MAX_LOG_TEXT_BYTES = 2 * 1024 * 1024;

const LOG_BATCH_INTERVAL_MS = 50;
const TRUNCATION_MARKER = "\n[log message truncated]";

export type AppLogLevel = "trace" | "debug" | "info" | "warn" | "error";
export type LogConnectionStatus = "connecting" | "live" | "unavailable";

export interface AppLogEntry {
  level: AppLogLevel;
  message: string;
}

interface LogStreamValue {
  clear: () => void;
  entries: AppLogEntry[];
  status: LogConnectionStatus;
}

const LogStreamContext = createContext<LogStreamValue | null>(null);

function normalizeLevel(level: string): AppLogLevel {
  switch (level.toLowerCase()) {
    case "trace":
    case "debug":
    case "warn":
    case "error":
      return level.toLowerCase() as AppLogLevel;
    default:
      return "info";
  }
}

function levelFromTauri(level: LogLevel): AppLogLevel {
  switch (level) {
    case LogLevel.Trace:
      return "trace";
    case LogLevel.Debug:
      return "debug";
    case LogLevel.Warn:
      return "warn";
    case LogLevel.Error:
      return "error";
    default:
      return "info";
  }
}

export function logMessageBytes(message: string) {
  return message.length * 2;
}

export function truncateLogMessage(message: string) {
  if (logMessageBytes(message) <= MAX_LOG_MESSAGE_BYTES) {
    return message;
  }

  const markerCodeUnits = TRUNCATION_MARKER.length;
  let end = Math.max(0, MAX_LOG_MESSAGE_BYTES / 2 - markerCodeUnits);
  const lastCodeUnit = message.charCodeAt(end - 1);
  if (lastCodeUnit >= 0xd800 && lastCodeUnit <= 0xdbff) {
    end -= 1;
  }
  return `${message.slice(0, end)}${TRUNCATION_MARKER}`;
}

function normalizeEntry(entry: AppLogEntry): AppLogEntry {
  const message = truncateLogMessage(entry.message);
  return message === entry.message ? entry : { ...entry, message };
}

function capEntries(entries: AppLogEntry[]) {
  const retained: AppLogEntry[] = [];
  let retainedBytes = 0;

  for (let index = entries.length - 1; index >= 0 && retained.length < MAX_LOG_ENTRIES; index -= 1) {
    const entry = normalizeEntry(entries[index]);
    const entryBytes = logMessageBytes(entry.message) + logMessageBytes(entry.level);
    if (retained.length > 0 && retainedBytes + entryBytes > MAX_LOG_TEXT_BYTES) {
      break;
    }
    retained.push(entry);
    retainedBytes += entryBytes;
  }

  retained.reverse();
  return retained;
}

function entriesMatch(left: AppLogEntry, right: AppLogEntry) {
  return left.level === right.level && left.message === right.message;
}

function mergeHistoryWithLive(history: AppLogEntry[], live: AppLogEntry[]) {
  if (live.length === 0) {
    return capEntries(history);
  }
  let overlap = 0;
  for (let historyStart = 0; historyStart < history.length; historyStart += 1) {
    if (!entriesMatch(history[historyStart], live[0])) {
      continue;
    }
    let matchLength = 0;
    while (
      historyStart + matchLength < history.length &&
      matchLength < live.length &&
      entriesMatch(history[historyStart + matchLength], live[matchLength])
    ) {
      matchLength += 1;
    }
    overlap = Math.max(overlap, matchLength);
  }
  return capEntries([...history, ...live.slice(overlap)]);
}

export function LogStreamProvider({ children }: { children: React.ReactNode }) {
  const [entries, setEntries] = useState<AppLogEntry[]>([]);
  const [status, setStatus] = useState<LogConnectionStatus>("connecting");

  useEffect(() => {
    let cancelled = false;
    let historyLoaded = false;
    let flushTimer: number | undefined;
    let liveBatch: AppLogEntry[] = [];
    let unlisten: (() => void) | undefined;
    let pendingEntries: AppLogEntry[] = [];

    function flushLiveBatch() {
      flushTimer = undefined;
      if (cancelled || liveBatch.length === 0) {
        liveBatch = [];
        return;
      }
      const batch = liveBatch;
      liveBatch = [];
      if (!historyLoaded) {
        pendingEntries = capEntries([...pendingEntries, ...batch]);
        return;
      }
      setEntries((previous) => capEntries([...previous, ...batch]));
    }

    function queueLiveEntry(entry: AppLogEntry) {
      liveBatch.push(entry);
      if (flushTimer === undefined) {
        flushTimer = window.setTimeout(flushLiveBatch, LOG_BATCH_INTERVAL_MS);
      }
    }

    async function connect() {
      try {
        unlisten = await attachLogger(({ level, message }) => {
          if (cancelled) {
            return;
          }
          queueLiveEntry(normalizeEntry({ level: levelFromTauri(level), message }));
        });
        if (cancelled) {
          unlisten();
          return;
        }
        setStatus("live");
      } catch {
        if (!cancelled) {
          setStatus("unavailable");
        }
      }

      try {
        const history = (await taurpc.getAppLogs()).map((entry) => ({
          level: normalizeLevel(entry.level),
          message: truncateLogMessage(entry.message),
        }));
        if (!cancelled) {
          setEntries(mergeHistoryWithLive(history, pendingEntries));
        }
      } catch {
        if (!cancelled && pendingEntries.length > 0) {
          setEntries(capEntries(pendingEntries));
        }
      }
      historyLoaded = true;
    }

    void connect();
    return () => {
      cancelled = true;
      if (flushTimer !== undefined) {
        window.clearTimeout(flushTimer);
      }
      unlisten?.();
    };
  }, []);

  const clear = useCallback(() => {
    setEntries([]);
  }, []);
  const value = useMemo(() => ({ clear, entries, status }), [clear, entries, status]);

  return <LogStreamContext value={value}>{children}</LogStreamContext>;
}

export function useLogStream() {
  const value = useContext(LogStreamContext);
  if (!value) {
    throw new Error("useLogStream must be used inside LogStreamProvider");
  }
  return value;
}
