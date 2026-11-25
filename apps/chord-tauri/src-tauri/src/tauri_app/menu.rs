use crate::app::AppHandleExt;
#[cfg(target_os = "macos")]
use crate::constants::CHECK_FOR_UPDATES_MENU_LABEL;
use crate::constants::{CHECK_FOR_UPDATES_MENU_ID, SHOW_SETTINGS_WINDOW_MENU_ID};
use tauri::menu::{Menu, MenuEvent};
#[cfg(target_os = "macos")]
use tauri::menu::{MenuItem, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::{AppHandle, Runtime};

pub fn build_app_menu<R: Runtime>(handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;

    #[cfg(target_os = "macos")]
    if let Some(app_menu) = menu.items()?.first().and_then(|item| item.as_submenu()) {
        let check_for_updates = MenuItem::with_id(
            handle,
            CHECK_FOR_UPDATES_MENU_ID,
            CHECK_FOR_UPDATES_MENU_LABEL,
            true,
            None::<&str>,
        )?;
        // Tauri's default macOS menu starts with the app submenu, with About first.
        app_menu.insert(&check_for_updates, 1)?;
    }

    #[cfg(target_os = "macos")]
    if let Some(window_menu) = menu
        .get(WINDOW_SUBMENU_ID)
        .and_then(|item| item.as_submenu().cloned())
    {
        let separator = PredefinedMenuItem::separator(handle)?;
        let show_settings_item = MenuItem::with_id(
            handle,
            SHOW_SETTINGS_WINDOW_MENU_ID,
            "Show Settings Window",
            true,
            None::<&str>,
        )?;

        window_menu.append_items(&[&separator, &show_settings_item])?;
    }

    Ok(menu)
}

pub fn handle_menu_event<R: Runtime>(handle: &AppHandle<R>, event: &MenuEvent) {
    match event.id().as_ref() {
        CHECK_FOR_UPDATES_MENU_ID => handle.app_state().app_updater().check_from_menu(),
        SHOW_SETTINGS_WINDOW_MENU_ID => {
            let settings = handle.app_state().settings();
            if let Err(error) = settings.ui.open() {
                log::error!("Failed to show settings window from app menu: {error}");
            }
        }
        _ => {}
    }
}
