use super::protocol::HostRequest;
use crate::app::AppHandleExt;
use crate::app::global_hotkey_store::GlobalHotkeyStoreEntry;
use crate::constants::GLOBAL_HOTKEYS_POOL;
use crate::models::{ShortcutChordAction, SimulatedShortcut};
use std::collections::HashSet;

/// The parent's narrow state/input bridge; no package function or native module executes here.
pub(crate) fn handle_host_request(
    handle: &tauri::AppHandle,
    request: HostRequest,
    pressed: &mut HashSet<String>,
    mouse: &mut crate::native_input::MouseSession,
) -> anyhow::Result<serde_json::Value> {
    use serde_json::Value as Json;
    // Workers can now call these APIs concurrently; allocation and storage must stay atomic.
    static HOST_REQUEST_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    let _host_guard = HOST_REQUEST_LOCK.lock();
    let input = match &request {
        HostRequest::Press(key) => Some((key, true, false)),
        HostRequest::Release(key) => Some((key, false, true)),
        HostRequest::Tap(key) => Some((key, true, true)),
        _ => None,
    };
    if let Some((key, down, up)) = input {
        let shortcut = key.parse::<SimulatedShortcut>()?;
        let state = handle.app_state();
        let runner = state.chord_action_task_runner();
        let action = ShortcutChordAction {
            simulated_shortcut: shortcut,
        };
        if down {
            pressed.insert(key.clone());
            runner.shortcut.simulate_shortcut_actions(
                runner
                    .shortcut
                    .get_start_simulated_shortcut_actions(&action, 1),
            )?;
        }
        if up {
            runner.shortcut.simulate_shortcut_actions(
                runner.shortcut.get_end_simulated_shortcut_actions(&action),
            )?;
            pressed.remove(key);
        }
        return Ok(Json::Null);
    }
    match request {
        HostRequest::Mouse(options) => {
            mouse.apply(&options)?;
            Ok(Json::Null)
        }
        HostRequest::Press(key) | HostRequest::Tap(key) | HostRequest::Release(key) => {
            anyhow::bail!("unhandled input operation: {key}")
        }
        HostRequest::GetGlobalHotkey {
            bundle_id,
            hotkey_id,
        } => {
            let shortcut = handle
                .app_state()
                .global_hotkey_store()
                .entries()?
                .into_iter()
                .find_map(|(shortcut, entry)| {
                    (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id)
                        .then_some(shortcut)
                });
            Ok(serde_json::to_value(shortcut)?)
        }
        HostRequest::RegisterGlobalHotkey {
            bundle_id,
            hotkey_id,
        } => {
            let state = handle.app_state();
            let store = state.global_hotkey_store();
            let all = store.entries()?;
            if let Some(shortcut) = all.iter().find_map(|(shortcut, entry)| {
                (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id)
                    .then_some(shortcut.clone())
            }) {
                return Ok(Json::String(shortcut));
            }
            let used: HashSet<_> = all.into_keys().collect();
            let Some(shortcut) = GLOBAL_HOTKEYS_POOL
                .iter()
                .map(|shortcut| shortcut.serialize())
                .find(|shortcut| !used.contains(shortcut))
            else {
                return Ok(Json::Null);
            };
            store.set(
                &shortcut,
                GlobalHotkeyStoreEntry {
                    bundle_id,
                    hotkey_id,
                },
            )?;
            Ok(Json::String(shortcut))
        }
        HostRequest::SetAppNeedsRelaunch {
            bundle_id,
            needs_relaunch,
        } => {
            handle
                .app_state()
                .desktop_app_manager()
                .set_app_needs_relaunch(&bundle_id, needs_relaunch)?;
            Ok(Json::Null)
        }
        #[cfg(not(target_os = "macos"))]
        HostRequest::AppWindows { app_id } => {
            let pids = crate::app::desktop_app::process_ids(handle, &app_id);
            Ok(serde_json::to_value(
                crate::native_input::get_process_windows(&pids)?,
            )?)
        }
        #[cfg(not(target_os = "macos"))]
        HostRequest::ScreenBounds => Ok(serde_json::to_value(
            crate::native_input::get_screen_bounds()?,
        )?),
        HostRequest::AppProcessIds { app_id } => {
            #[cfg(target_os = "macos")]
            let pids: Vec<i32> = crate::app::desktop_app::running_apps_with_id(handle, &app_id)
                .iter()
                .map(|app| app.processIdentifier())
                .collect();
            #[cfg(not(target_os = "macos"))]
            let pids = crate::app::desktop_app::process_ids(handle, &app_id);
            Ok(serde_json::to_value(pids)?)
        }
    }
}
