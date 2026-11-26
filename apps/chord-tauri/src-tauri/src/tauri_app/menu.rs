use crate::app::AppHandleExt;
#[cfg(target_os = "macos")]
use crate::constants::CHECK_FOR_UPDATES_MENU_LABEL;
use crate::constants::{
    CHECK_FOR_UPDATES_MENU_ID, CLOSE_SETTINGS_MENU_ID, QUIT_MENU_ID, SHOW_SETTINGS_WINDOW_MENU_ID,
};
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

        // Chord lives in the menu bar, like Homerow: its chords and Hint Panel are the app,
        // and Settings is only a window of it. So ⌘Q closes Settings and Chord keeps
        // running in the background; quitting for good is ⌥⌘Q here, or Quit Chord in the
        // menu bar icon's menu. The Dock's Quit and logging out still quit as usual.
        let items = app_menu.items()?;
        if let Some(quit) = items.last().filter(|item| {
            item.as_predefined_menuitem()
                .and_then(|item| item.text().ok())
                .is_some_and(|text| text.starts_with("Quit"))
        }) {
            app_menu.remove(quit)?;
        }
        let close_settings = MenuItem::with_id(
            handle,
            CLOSE_SETTINGS_MENU_ID,
            "Close Settings and Keep Running",
            true,
            Some("CmdOrCtrl+Q"),
        )?;
        let quit = MenuItem::with_id(
            handle,
            QUIT_MENU_ID,
            "Quit Chord",
            true,
            Some("Alt+CmdOrCtrl+Q"),
        )?;
        app_menu.append_items(&[&close_settings, &quit])?;
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
        CLOSE_SETTINGS_MENU_ID => {
            if let Err(error) = handle.app_state().settings().ui.close() {
                log::error!("Failed to close the settings window: {error:#}");
            }
        }
        QUIT_MENU_ID => handle.exit(0),
        _ => {}
    }
}
