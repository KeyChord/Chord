use crate::constants::{QUIT_MENU_ID, RELOAD_CONFIGS_MENU_ID, SETTINGS_MENU_ID, UPDATE_MENU_ID};
use crate::state::AppUpdateState;
use crate::tauri_app::scripting::reload_configs;
use tauri::{
    AppHandle, Manager, Wry,
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
};
use crate::app::AppHandleExt;

#[cfg(debug_assertions)]
use crate::constants::OPEN_INSPECTOR_MENU_ID;

pub const TRAY_ID: &str = "chord-tray";
// Black-on-transparent template image, 36×36 px; tray icons are drawn 18 pt
// tall with their width following the aspect ratio, so this is a square 2×
// bitmap. The glyph is 14 pt, the visual size of the system's menu bar icons.
const TRAY_ICON_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tray.png"));

fn load_tray_icon() -> tauri::Result<Image<'static>> {
    Image::from_bytes(TRAY_ICON_BYTES).map(|image| image.to_owned())
}

/// The label for the update item and whether it can be clicked, if there's an update to show.
fn update_menu_item(state: &AppUpdateState) -> Option<(String, bool)> {
    match state {
        AppUpdateState::Idle => None,
        AppUpdateState::Available { version } => Some((format!("Update to Chord {version}"), true)),
        AppUpdateState::Downloading { version, .. } => {
            Some((format!("Downloading Chord {version}…"), false))
        }
        AppUpdateState::Installing { version } => {
            Some((format!("Installing Chord {version}…"), false))
        }
        AppUpdateState::Ready { .. } => Some(("Restart to Finish Updating".to_string(), true)),
    }
}

fn build_menu(handle: &AppHandle, update: &AppUpdateState) -> tauri::Result<Menu<Wry>> {
    let update_item = update_menu_item(update)
        .map(|(label, enabled)| {
            MenuItemBuilder::with_id(UPDATE_MENU_ID, label)
                .enabled(enabled)
                .build(handle)
        })
        .transpose()?;

    let mut menu = MenuBuilder::new(handle);
    if let Some(item) = &update_item {
        menu = menu.item(item).separator();
    }
    menu = menu
        .text(SETTINGS_MENU_ID, "Show Settings")
        .text(RELOAD_CONFIGS_MENU_ID, "Reload Configs");

    #[cfg(debug_assertions)]
    {
        let inspector_text = if handle
            .get_webview_window("chords")
            .is_some_and(|w| w.is_visible().unwrap_or(false))
        {
            "Close Inspector"
        } else {
            "Open Inspector"
        };
        menu = menu.text(OPEN_INSPECTOR_MENU_ID, inspector_text);
    }

    menu.separator().text(QUIT_MENU_ID, "Quit Chord").build()
}

/// Rebuilds the menu, because a macOS menu item can't be shown or hidden once it exists.
pub fn refresh_menu(handle: &AppHandle, update: &AppUpdateState) -> tauri::Result<()> {
    let Some(tray) = handle.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(build_menu(handle, update)?))
}

pub fn create_tray(handle: AppHandle) -> tauri::Result<()> {
    let menu = build_menu(&handle, &AppUpdateState::Idle)?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Chord")
        .show_menu_on_left_click(true)
        .icon(load_tray_icon()?)
        .on_menu_event(|handle, event| match event.id().as_ref() {
            UPDATE_MENU_ID => handle.app_state().app_updater().on_menu_item_clicked(),
            SETTINGS_MENU_ID => {
                let settings = handle.app_state().settings();
                if let Err(e) = settings.ui.open() {
                    log::error!("Failed to show settings window: {e}");
                }
            }
            RELOAD_CONFIGS_MENU_ID => {
                reload_configs(handle.clone());
            }
            #[cfg(debug_assertions)]
            OPEN_INSPECTOR_MENU_ID => {
                let chord_mode_manager = handle.app_state().chord_mode_manager();
                match chord_mode_manager.panel.toggle_inspector() {
                    Ok(is_open) => {
                        let Some(menu) = handle.menu() else {
                            log::error!("Failed to update inspector menu item: no menu set");
                            return;
                        };
                        if let Some(item) = menu.get(OPEN_INSPECTOR_MENU_ID) {
                            if let Some(text_item) = item.as_menuitem() {
                                let _ = text_item.set_text(if is_open {
                                    "Close Inspector"
                                } else {
                                    "Open Inspector"
                                });
                            }
                        }
                    }
                    Err(error) => {
                        log::error!("Failed to toggle inspector: {error}");
                    }
                }
            }
            QUIT_MENU_ID => {
                handle.exit(0);
            }
            _ => {}
        });

    #[cfg(target_os = "macos")]
    {
        tray = tray.icon_as_template(true);
    }

    tray.build(&handle)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_update_item_is_clickable_only_when_there_is_something_to_do() {
        let version = "0.2.4".to_string();
        assert_eq!(update_menu_item(&AppUpdateState::Idle), None);
        assert_eq!(
            update_menu_item(&AppUpdateState::Available {
                version: version.clone()
            }),
            Some(("Update to Chord 0.2.4".to_string(), true))
        );
        assert_eq!(
            update_menu_item(&AppUpdateState::Downloading {
                version: version.clone(),
                percent: Some(40)
            }),
            Some(("Downloading Chord 0.2.4…".to_string(), false))
        );
        assert_eq!(
            update_menu_item(&AppUpdateState::Installing {
                version: version.clone()
            }),
            Some(("Installing Chord 0.2.4…".to_string(), false))
        );
        assert_eq!(
            update_menu_item(&AppUpdateState::Ready { version }),
            Some(("Restart to Finish Updating".to_string(), true))
        );
    }
}
