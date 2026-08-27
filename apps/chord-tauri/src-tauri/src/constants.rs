use crate::models::{Key, KeyCombination, KeyCombinationModifiers};
use anyhow::{Context, bail};
use jsonc_parser::{JsonValue, ParseOptions};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

pub const SETTINGS_MENU_ID: &str = "settings";
pub const SHOW_SETTINGS_WINDOW_MENU_ID: &str = "show-settings-window";
pub const RELOAD_CONFIGS_MENU_ID: &str = "reload-configs";
pub const OPEN_INSPECTOR_MENU_ID: &str = "open-inspector";
pub const QUIT_MENU_ID: &str = "quit";

/// Native handlers run arbitrary code that cannot be preempted; exceeding this kills and
/// restarts the whole native host, so it is generous.
pub const NATIVE_INVOCATION_TIMEOUT: Duration = Duration::from_secs(30);
/// Handshake and generation-load budget for a freshly spawned native host.
pub const NATIVE_HOST_READY_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a retiring host gets to exit cleanly before it is killed.
pub const NATIVE_HOST_SHUTDOWN_GRACE: Duration = Duration::from_secs(2);
/// A handler that crashes/hangs the host this many times within the window is disabled until
/// the next package reload.
pub const NATIVE_CRASH_LOOP_LIMIT: usize = 3;
pub const NATIVE_CRASH_LOOP_WINDOW: Duration = Duration::from_secs(10);

pub static GLOBAL_HOTKEYS_POOL: LazyLock<Vec<KeyCombination>> =
    LazyLock::new(|| load_hotkeys().expect("failed to load GLOBAL_HOTKEYS_POOL"));

fn load_hotkeys() -> anyhow::Result<Vec<KeyCombination>> {
    let data = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/global-hotkey-pool.jsonc"
    ));

    let parsed = jsonc_parser::parse_to_value(data, &ParseOptions::default())
        .context("failed to parse jsonc")?;

    let array = match parsed {
        Some(JsonValue::Array(arr)) => arr,
        _ => bail!("expected top-level array"),
    };

    let mut result = Vec::new();

    for (i, item) in array.into_iter().enumerate() {
        let object = match item {
            JsonValue::Object(obj) => obj,
            _ => bail!("item {i} is not an object"),
        };

        let key = object
            .get_string("key")
            .context(format!("item {i}: missing 'key'"))?
            .to_string();

        let modifiers_obj = object
            .get_object("mod")
            .context(format!("item {i}: missing 'mod'"))?;

        let parse_flag = |k: &str| -> anyhow::Result<bool> {
            let val = modifiers_obj
                .get_number(k)
                .with_context(|| format!("item {i}: missing 'mod.{k}'"))?;

            Ok(val == "1")
        };

        let modifiers = KeyCombinationModifiers {
            meta: parse_flag("m")?,
            ctrl: parse_flag("c")?,
            alt: parse_flag("a")?,
            shift: parse_flag("s")?,
        };

        result.push(KeyCombination {
            key: Key::from_str(&key)?,
            modifiers,
        });
    }

    Ok(result)
}
