use crate::constants::{QUIT_MENU_ID, RELOAD_CONFIGS_MENU_ID, SETTINGS_MENU_ID};
use crate::tauri_app::scripting::reload_configs;
use crate::tauri_app::settings::show_settings_window;
use tauri::{AppHandle, image::Image, menu::MenuBuilder, tray::TrayIconBuilder};

#[cfg(debug_assertions)]
use crate::constants::OPEN_INSPECTOR_MENU_ID;
#[cfg(debug_assertions)]
use crate::settings::open_chords_inspector;

pub const TRAY_ID: &str = "chord-tray";
const TRAY_ICON_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/32x32.png"));

fn load_tray_icon() -> tauri::Result<Image<'static>> {
    Image::from_bytes(TRAY_ICON_BYTES).map(|image| image.to_owned())
}

pub fn create_tray(handle: AppHandle) -> tauri::Result<()> {
    #[cfg(debug_assertions)]
    let menu = MenuBuilder::new(&handle)
        .text(SETTINGS_MENU_ID, "Show Settings")
        .text(RELOAD_CONFIGS_MENU_ID, "Reload Configs")
        .text(OPEN_INSPECTOR_MENU_ID, "Open Inspector");

    #[cfg(not(debug_assertions))]
    let menu = MenuBuilder::new(&handle)
        .text(SETTINGS_MENU_ID, "Show Settings")
        .text(RELOAD_CONFIGS_MENU_ID, "Reload Configs");

    let menu = menu.separator().text(QUIT_MENU_ID, "Quit").build()?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Chord")
        .show_menu_on_left_click(true)
        .icon(load_tray_icon()?)
        .on_menu_event(|handle, event| match event.id().as_ref() {
            SETTINGS_MENU_ID => {
                if let Err(e) = show_settings_window(handle.clone()) {
                    log::error!("Failed to show settings window: {e}");
                }
            }
            RELOAD_CONFIGS_MENU_ID => {
                reload_configs(handle.clone());
            }
            #[cfg(debug_assertions)]
            OPEN_INSPECTOR_MENU_ID => {
                if let Err(error) = open_chords_inspector(handle.clone()) {
                    log::error!("Failed to open inspector: {error}");
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
