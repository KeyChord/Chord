use crate::feature::app_handle_ext::AppHandleExt;
use crate::tauri_app::settings::show_settings_window;
use anyhow::{Context, Result, bail};
use std::sync::OnceLock;
use tauri::AppHandle;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptCommand {
    OpenSettings,
    ReloadConfigs,
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
        ScriptCommand::OpenSettings => show_settings_window(handle)?,
        ScriptCommand::ReloadConfigs => reload_configs(handle),
    }

    Ok(())
}

pub fn reload_configs(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let chord_registry = handle.app_chord_registry();
        if let Err(error) = chord_registry.reload().await {
            log::error!("Failed to reload configs: {error}");
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
    use super::{ScriptCommand, normalize_command, parse_command};

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
}
