use crate::startup::APP_STATE_STORE_PATH;
use anyhow::{Context, Result};
use log::{Level, LevelFilter, Metadata};
use parking_lot::Mutex;
use serde::Serialize;
use specta::Type;
use std::collections::VecDeque;
use std::fmt::{Arguments, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{LazyLock, OnceLock};
use std::thread;
use tauri::{AppHandle, Emitter};
use tauri_plugin_log::{Target, TargetKind, fern};
use tauri_plugin_store::StoreExt;

const ENV_VAR: &str = "RUST_LOG";
const MAX_HISTORY_ENTRIES: usize = 1_000;
const MAX_HISTORY_BYTES: usize = 2 * 1024 * 1024;
const MAX_LOG_MESSAGE_BYTES: usize = 64 * 1024;
const WEBVIEW_LOG_QUEUE_CAPACITY: usize = 64;
const WEBVIEW_LOG_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
const TRUNCATION_MARKER: &str = "\n[log message truncated]";
const STORE_KEY: &str = "logLevel";

static ACTIVE_LEVEL: AtomicU8 = AtomicU8::new(LevelFilter::Info as u8);
static DROPPED_WEBVIEW_LOGS: AtomicU64 = AtomicU64::new(0);
static LOG_HISTORY: LazyLock<Mutex<LogHistory>> =
    LazyLock::new(|| Mutex::new(LogHistory::default()));
static WEBVIEW_LOG_SENDER: OnceLock<SyncSender<WebviewLogEntry>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppLogEntry {
    pub level: String,
    pub message: String,
}

#[derive(Default)]
struct LogHistory {
    bytes: usize,
    entries: VecDeque<AppLogEntry>,
}

impl LogHistory {
    fn push(&mut self, entry: AppLogEntry) {
        let entry_bytes = log_entry_bytes(&entry);
        while !self.entries.is_empty()
            && (self.entries.len() >= MAX_HISTORY_ENTRIES
                || self.bytes.saturating_add(entry_bytes) > MAX_HISTORY_BYTES)
        {
            if let Some(removed) = self.entries.pop_front() {
                self.bytes = self.bytes.saturating_sub(log_entry_bytes(&removed));
            }
        }
        self.bytes = self.bytes.saturating_add(entry_bytes);
        self.entries.push_back(entry);
    }
}

#[derive(Clone, Serialize)]
struct WebviewLogEntry {
    level: u16,
    message: String,
}

struct LimitedWriter {
    limit: usize,
    text: String,
    truncated: bool,
}

impl LimitedWriter {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            text: String::with_capacity(limit.min(4 * 1024)),
            truncated: false,
        }
    }

    fn finish(mut self) -> String {
        if self.truncated {
            self.text.push_str(TRUNCATION_MARKER);
        }
        self.text
    }
}

impl Write for LimitedWriter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let remaining = self.limit.saturating_sub(self.text.len());
        if value.len() <= remaining {
            self.text.push_str(value);
            return Ok(());
        }

        let mut end = remaining.min(value.len());
        while end > 0 && !value.is_char_boundary(end) {
            end -= 1;
        }
        self.text.push_str(&value[..end]);
        self.truncated = true;
        Ok(())
    }
}

pub fn bounded_message(arguments: &Arguments<'_>) -> String {
    let content_limit = MAX_LOG_MESSAGE_BYTES.saturating_sub(TRUNCATION_MARKER.len());
    let mut writer = LimitedWriter::new(content_limit);
    let _ = std::fmt::write(&mut writer, *arguments);
    writer.finish()
}

pub fn history_target() -> Target {
    let dispatch = fern::Dispatch::new().chain(fern::Output::call(|record| {
        let mut history = LOG_HISTORY.lock();
        history.push(AppLogEntry {
            level: record.level().as_str().to_ascii_lowercase(),
            message: bounded_message(record.args()),
        });
    }));

    Target::new(TargetKind::Dispatch(dispatch))
}

pub fn webview_target() -> Target {
    let dispatch = fern::Dispatch::new().chain(fern::Output::call(|record| {
        let Some(sender) = WEBVIEW_LOG_SENDER.get() else {
            return;
        };
        let entry = WebviewLogEntry {
            level: webview_level(record.level()),
            message: bounded_message(record.args()),
        };
        if let Err(TrySendError::Full(_)) = sender.try_send(entry) {
            DROPPED_WEBVIEW_LOGS.fetch_add(1, Ordering::Relaxed);
        }
    }));

    Target::new(TargetKind::Dispatch(dispatch))
}

pub fn recent_entries() -> Vec<AppLogEntry> {
    LOG_HISTORY.lock().entries.iter().cloned().collect()
}

pub fn initialize_early() {
    if let Some(level) = environment_level() {
        set_level(level);
    }
}

pub fn initialize(handle: &AppHandle) -> Result<()> {
    initialize_webview_log_stream(handle.clone());

    let configured_level = match environment_level() {
        Some(level) => Some(level),
        None => persisted_level(handle)?,
    };
    let level = configured_level.unwrap_or(LevelFilter::Info);

    set_level(level);

    #[cfg(debug_assertions)]
    terminal::start(handle.clone());

    log::info!("Log level: {level}");
    Ok(())
}

fn initialize_webview_log_stream(handle: AppHandle) {
    let (sender, receiver) = sync_channel(WEBVIEW_LOG_QUEUE_CAPACITY);
    if WEBVIEW_LOG_SENDER.set(sender).is_err() {
        return;
    }

    thread::Builder::new()
        .name("webview-log".into())
        .spawn(move || {
            while let Ok(entry) = receiver.recv() {
                let dropped = DROPPED_WEBVIEW_LOGS.swap(0, Ordering::Relaxed);
                if dropped > 0 {
                    let _ = handle.emit(
                        "log://log",
                        WebviewLogEntry {
                            level: webview_level(Level::Warn),
                            message: format!(
                                "[WARN] logging - dropped {dropped} webview log entries because the bounded queue was full"
                            ),
                        },
                    );
                }
                let _ = handle.emit("log://log", entry);
                thread::sleep(WEBVIEW_LOG_MIN_INTERVAL);
            }
        })
        .expect("failed to spawn webview log worker");
}

fn log_entry_bytes(entry: &AppLogEntry) -> usize {
    entry.level.len().saturating_add(entry.message.len())
}

fn webview_level(level: Level) -> u16 {
    match level {
        Level::Trace => 1,
        Level::Debug => 2,
        Level::Info => 3,
        Level::Warn => 4,
        Level::Error => 5,
    }
}

pub fn enabled(metadata: &Metadata<'_>) -> bool {
    metadata.level() <= active_level()
}

fn active_level() -> LevelFilter {
    match ACTIVE_LEVEL.load(Ordering::Relaxed) {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}

fn set_level(level: LevelFilter) {
    ACTIVE_LEVEL.store(level as u8, Ordering::Relaxed);
}

fn environment_level() -> Option<LevelFilter> {
    std::env::var(ENV_VAR)
        .ok()
        .and_then(|value| LevelFilter::from_str(value.trim()).ok())
}

fn persisted_level(handle: &AppHandle) -> Result<Option<LevelFilter>> {
    let store = handle
        .store(APP_STATE_STORE_PATH)
        .context("failed to open app state store")?;
    let Some(value) = store.get(STORE_KEY) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .with_context(|| format!("app setting `{STORE_KEY}` must be a string"))?;
    let level = LevelFilter::from_str(value)
        .with_context(|| format!("invalid persisted log level `{value}`"))?;

    Ok(Some(level))
}

fn persist_level(handle: &AppHandle, level: LevelFilter) -> Result<()> {
    let store = handle
        .store(APP_STATE_STORE_PATH)
        .context("failed to open app state store")?;
    store.set(STORE_KEY, level.as_str().to_ascii_lowercase());
    store.save().context("failed to save app state store")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_a_single_formatted_message_on_a_utf8_boundary() {
        let oversized = "🧠".repeat(MAX_LOG_MESSAGE_BYTES);
        let message = bounded_message(&format_args!("{oversized}"));

        assert!(message.len() <= MAX_LOG_MESSAGE_BYTES);
        assert!(message.ends_with(TRUNCATION_MARKER));
        assert!(std::str::from_utf8(message.as_bytes()).is_ok());
    }

    #[test]
    fn history_enforces_entry_and_byte_budgets() {
        let mut history = LogHistory::default();
        for index in 0..(MAX_HISTORY_ENTRIES + 10) {
            history.push(AppLogEntry {
                level: "info".into(),
                message: format!("{index}:{}", "x".repeat(MAX_LOG_MESSAGE_BYTES)),
            });
        }

        assert!(history.entries.len() < MAX_HISTORY_ENTRIES);
        assert!(history.bytes <= MAX_HISTORY_BYTES);
        assert_eq!(
            history.bytes,
            history.entries.iter().map(log_entry_bytes).sum::<usize>()
        );
    }
}

#[cfg(debug_assertions)]
mod terminal {
    use super::{active_level, persist_level, set_level};
    use log::LevelFilter;
    use std::fs;
    use std::io::{self, BufRead};
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;
    use tauri::AppHandle;

    const CONTROL_FILE_ENV_VAR: &str = "CHORD_LOG_CONTROL_FILE";
    static STARTED: AtomicBool = AtomicBool::new(false);

    pub fn start(handle: AppHandle) {
        if STARTED.swap(true, Ordering::Relaxed) {
            return;
        }

        println!(
            "[log-control] level: {}. Commands: `log debug`, `log info`, `log trace`, `log toggle`, `log status`.",
            active_level()
        );

        if let Some(path) = std::env::var_os(CONTROL_FILE_ENV_VAR).map(PathBuf::from) {
            thread::spawn(move || watch_control_file(&handle, path));
        } else {
            thread::spawn(move || read_stdin(&handle));
        }
    }

    fn read_stdin(handle: &AppHandle) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => handle_line(handle, &line),
                Err(error) => {
                    eprintln!("[log-control] failed to read terminal input: {error}");
                    break;
                }
            }
        }
    }

    fn watch_control_file(handle: &AppHandle, path: PathBuf) {
        let mut previous_payload = fs::read_to_string(&path).unwrap_or_default();

        loop {
            thread::sleep(Duration::from_millis(100));
            let Ok(payload) = fs::read_to_string(&path) else {
                continue;
            };
            if payload == previous_payload {
                continue;
            }
            previous_payload.clone_from(&payload);

            let command = payload
                .split_once('\t')
                .map(|(_, command)| command)
                .unwrap_or(&payload);
            handle_line(handle, command.trim());
        }
    }

    fn handle_line(handle: &AppHandle, line: &str) {
        let Some(command) = parse_command(line) else {
            return;
        };

        match command {
            Ok(Command::Status) => {
                println!("[log-control] level: {}", active_level());
            }
            Ok(Command::Help) => print_help(),
            Ok(Command::Toggle) => {
                let level = if active_level() >= LevelFilter::Debug {
                    LevelFilter::Info
                } else {
                    LevelFilter::Debug
                };
                apply_level(handle, level);
            }
            Ok(Command::Set(level)) => apply_level(handle, level),
            Err(error) => {
                eprintln!("[log-control] {error}");
                print_help();
            }
        }
    }

    fn apply_level(handle: &AppHandle, level: LevelFilter) {
        set_level(level);
        match persist_level(handle, level) {
            Ok(()) => println!("[log-control] log level changed to {level} and persisted"),
            Err(error) => {
                eprintln!("[log-control] level set to {level}, but could not persist it: {error:#}")
            }
        }
    }

    fn print_help() {
        println!(
            "[log-control] use `log <off|error|warn|info|debug|trace>`, `log toggle`, or `log status`"
        );
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        Status,
        Help,
        Toggle,
        Set(LevelFilter),
    }

    fn parse_command(line: &str) -> Option<Result<Command, String>> {
        let mut words = line.split_whitespace();
        if words.next()? != "log" {
            return None;
        }

        let command = words.next().unwrap_or("status");
        if words.next().is_some() {
            return Some(Err("too many arguments".to_string()));
        }

        Some(match command {
            "status" => Ok(Command::Status),
            "help" => Ok(Command::Help),
            "toggle" => Ok(Command::Toggle),
            value => LevelFilter::from_str(value)
                .map(Command::Set)
                .map_err(|_| format!("unknown log level or command `{value}`")),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::{Command, parse_command};
        use log::LevelFilter;

        #[test]
        fn parses_log_commands() {
            assert_eq!(parse_command("log"), Some(Ok(Command::Status)));
            assert_eq!(
                parse_command("log debug"),
                Some(Ok(Command::Set(LevelFilter::Debug)))
            );
            assert_eq!(parse_command("log toggle"), Some(Ok(Command::Toggle)));
            assert_eq!(parse_command("log status"), Some(Ok(Command::Status)));
        }

        #[test]
        fn ignores_unrelated_terminal_input() {
            assert_eq!(parse_command("debug"), None);
            assert_eq!(parse_command(""), None);
        }

        #[test]
        fn rejects_invalid_log_commands() {
            assert!(matches!(parse_command("log verbose"), Some(Err(_))));
            assert!(matches!(parse_command("log debug now"), Some(Err(_))));
        }
    }
}
