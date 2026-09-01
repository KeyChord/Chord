use crate::app::AppHandleExt;
use crate::tauri_app::play_failure_sound;
use anyhow::{Context, Result, bail};
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static PENDING_CHORDS: Mutex<PendingChords> = Mutex::new(PendingChords {
    ready: false,
    sequences: Vec::new(),
});

struct PendingChords {
    ready: bool,
    sequences: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptCommand {
    OpenSettings,
    ReloadConfigs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAppCommand {
    Chord(String),
}

pub fn init(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);

    #[cfg(target_os = "macos")]
    macos::init_url_handler();
}

pub fn handle_url(url: &str) -> Result<()> {
    let command = parse_command(url)?;
    let handle = APP_HANDLE
        .get()
        .cloned()
        .context("app handle is not initialized")?;

    match command {
        ScriptCommand::OpenSettings => {
            let settings = handle.app_state().settings();
            settings.ui.open()?
        }
        ScriptCommand::ReloadConfigs => reload_configs(handle),
    }

    Ok(())
}

pub fn handle_cli_app_command(handle: &AppHandle, command: CliAppCommand) -> Result<()> {
    match command {
        CliAppCommand::Chord(sequence) => execute_or_queue_chord(handle, sequence),
    }
}

pub fn cli_app_command_from_args(args: &[String]) -> Option<CliAppCommand> {
    match args.get(1..)? {
        [sequence] if !sequence.starts_with('-') => Some(CliAppCommand::Chord(sequence.clone())),
        [command, sequence] if command == "chord" || command == "__app-chord" => {
            Some(CliAppCommand::Chord(sequence.clone()))
        }
        _ => None,
    }
}

fn execute_or_queue_chord(handle: &AppHandle, sequence: String) -> Result<()> {
    {
        let mut pending = PENDING_CHORDS
            .lock()
            .map_err(|_| anyhow::anyhow!("pending chord queue is poisoned"))?;
        if !pending.ready {
            pending.sequences.push(sequence);
            return Ok(());
        }
    }

    handle
        .app_state()
        .chord_mode_manager()
        .execute_sequence(&sequence)
}

pub fn mark_chord_packages_ready(handle: &AppHandle) {
    let sequences = {
        let Ok(mut pending) = PENDING_CHORDS.lock() else {
            log::error!("Pending chord queue is poisoned");
            return;
        };
        pending.ready = true;
        std::mem::take(&mut pending.sequences)
    };

    for sequence in sequences {
        if let Err(error) = handle
            .app_state()
            .chord_mode_manager()
            .execute_sequence(&sequence)
        {
            log::error!("Failed to execute queued CLI chord `{sequence}`: {error:#}");
            play_failure_sound(handle);
        }
    }
}

pub fn reload_configs(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let chord_pm = handle.app_state().chord_package_manager();
        if let Err(e) = chord_pm.reload_all().await {
            log::error!("Failed to reload packages: {}", e);
        }
    });
}

fn parse_command(url: &str) -> Result<ScriptCommand> {
    let command = normalize_command(url)?;

    match command.as_str() {
        "settings" | "open-settings" | "show-settings" => Ok(ScriptCommand::OpenSettings),
        "reload-config" | "reload-configs" => Ok(ScriptCommand::ReloadConfigs),
        _ => bail!("Unsupported chord URL command: {command}"),
    }
}

fn normalize_command(url: &str) -> Result<String> {
    let remainder = url
        .strip_prefix("chord:")
        .context("URL must start with chord:")?;

    let remainder = remainder.strip_prefix("//").unwrap_or(remainder);
    let command = remainder
        .split_once('?')
        .map(|(command, _)| command)
        .unwrap_or(remainder)
        .trim_matches('/');

    if command.is_empty() {
        bail!("Missing chord URL command");
    }

    Ok(command.to_ascii_lowercase())
}

#[cfg(target_os = "macos")]
mod macos {
    use super::handle_url;
    use objc2::declare::ClassBuilder;
    use objc2::runtime::{AnyClass, AnyObject, NSObject, Sel};
    use objc2::{ClassType, msg_send, sel};
    use objc2_foundation::NSString;
    use std::sync::OnceLock;

    const INTERNET_EVENT_CLASS: u32 = u32::from_be_bytes(*b"GURL");
    const GET_URL_EVENT_ID: u32 = u32::from_be_bytes(*b"GURL");
    const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    static URL_HANDLER_INIT: OnceLock<()> = OnceLock::new();
    static URL_HANDLER_INSTANCE: OnceLock<usize> = OnceLock::new();

    pub fn init_url_handler() {
        if URL_HANDLER_INIT.set(()).is_err() {
            return;
        }

        let mut builder = ClassBuilder::new(c"ChordUrlEventHandler", NSObject::class())
            .expect("a class with name ChordUrlEventHandler likely already exists");

        unsafe extern "C" fn init(this: *mut NSObject, _sel: Sel) -> *mut NSObject {
            unsafe { msg_send![super(this, NSObject::class()), init] }
        }

        unsafe extern "C" fn handle_get_url_event(
            _this: *mut NSObject,
            _sel: Sel,
            event: *mut AnyObject,
            _reply_event: *mut AnyObject,
        ) {
            let Some(url) = apple_event_url(event) else {
                log::warn!("Ignoring chord URL event without a URL payload");
                return;
            };

            if let Err(error) = handle_url(&url) {
                log::error!("Failed to handle chord URL {url:?}: {error:#}");
            }
        }

        unsafe {
            builder.add_method(
                sel!(init),
                init as unsafe extern "C" fn(*mut NSObject, Sel) -> *mut NSObject,
            );
            builder.add_method(
                sel!(handleGetURLEvent:withReplyEvent:),
                handle_get_url_event
                    as unsafe extern "C" fn(*mut NSObject, Sel, *mut AnyObject, *mut AnyObject),
            );
        }

        let handler_class = builder.register();

        unsafe {
            let handler: *mut NSObject = msg_send![handler_class, alloc];
            let handler: *mut NSObject = msg_send![handler, init];
            let manager_class = AnyClass::get(c"NSAppleEventManager")
                .expect("NSAppleEventManager should exist on macOS");
            let manager: *mut AnyObject = msg_send![manager_class, sharedAppleEventManager];

            let _: () = msg_send![
                manager,
                setEventHandler: &*handler,
                andSelector: sel!(handleGetURLEvent:withReplyEvent:),
                forEventClass: INTERNET_EVENT_CLASS,
                andEventID: GET_URL_EVENT_ID
            ];

            let _ = URL_HANDLER_INSTANCE.set(handler as usize);
        }
    }

    fn apple_event_url(event: *mut AnyObject) -> Option<String> {
        unsafe {
            let descriptor: *mut AnyObject =
                msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT];
            let descriptor = descriptor.as_ref()?;
            let value: *mut NSString = msg_send![descriptor, stringValue];
            value.as_ref().map(NSString::to_string)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CliAppCommand, ScriptCommand, cli_app_command_from_args, normalize_command, parse_command,
    };

    #[test]
    fn parses_direct_scheme_command() {
        assert_eq!(
            parse_command("chord:reload-config").unwrap(),
            ScriptCommand::ReloadConfigs
        );
    }

    #[test]
    fn parses_host_style_command() {
        assert_eq!(
            parse_command("chord://settings").unwrap(),
            ScriptCommand::OpenSettings
        );
    }

    #[test]
    fn strips_query_parameters() {
        assert_eq!(
            normalize_command("chord://reload-configs?source=raycast").unwrap(),
            "reload-configs"
        );
    }

    #[test]
    fn parses_explicit_cli_chord() {
        assert_eq!(
            cli_app_command_from_args(&["/Applications/Chord".into(), "chord".into(), "fq".into()]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn parses_internal_detached_chord() {
        assert_eq!(
            cli_app_command_from_args(&[
                "/Applications/Chord".into(),
                "__app-chord".into(),
                "fq".into()
            ]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn parses_shorthand_cli_chord() {
        assert_eq!(
            cli_app_command_from_args(&["/Applications/Chord".into(), "fq".into()]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn ignores_non_chord_instance_arguments() {
        assert_eq!(cli_app_command_from_args(&["chord".into()]), None);
        assert_eq!(
            cli_app_command_from_args(&["chord".into(), "--autostart".into()]),
            None
        );
    }
}
