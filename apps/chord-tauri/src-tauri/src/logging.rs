use crate::startup::APP_STATE_STORE_PATH;
use anyhow::{Context, Result};
use log::{LevelFilter, Metadata};
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const ENV_VAR: &str = "RUST_LOG";
const STORE_KEY: &str = "logLevel";

static ACTIVE_LEVEL: AtomicU8 = AtomicU8::new(LevelFilter::Info as u8);

pub fn initialize_early() {
    if let Some(level) = environment_level() {
        set_level(level);
    }
}

pub fn initialize(handle: &AppHandle) -> Result<()> {
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
