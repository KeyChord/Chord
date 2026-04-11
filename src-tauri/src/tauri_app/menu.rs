use crate::constants::SHOW_SETTINGS_WINDOW_MENU_ID;
#[allow(unused_imports)]
use tauri::{
    AppHandle, Runtime,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, WINDOW_SUBMENU_ID},
};
use crate::app::AppHandleExt;

pub fn build_app_menu<R: Runtime>(handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;

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
        SHOW_SETTINGS_WINDOW_MENU_ID => {
            let settings = handle.app_state().settings();
            if let Err(error) = settings.ui.open() {
                log::error!("Failed to show settings window from app menu: {error}");
            }
        }
        _ => {}
    }
}
