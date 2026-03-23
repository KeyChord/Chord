use crate::input::{Key, KeyEvent};
use crate::mode::AppMode;
use crate::AppContext;
use anyhow::Result;
use keycode::KeyMappingCode;
use tauri::{AppHandle, Manager};
use crate::feature::AppChorder;

pub fn handle_key_event(handle: AppHandle, key_event: KeyEvent) -> Result<()> {
    let context = handle.state::<AppContext>();
    let app_mode = context.get_app_mode();
    let chorder = handle.state::<AppChorder>();

    match app_mode {
        AppMode::Chord => {
                chorder
                .handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::None => {
            let should_finalize_chord_mode =
                matches!(key_event, KeyEvent::Release(Key(KeyMappingCode::Space)))
                    && chorder.observable.state.get()?.is_idle();

            if should_finalize_chord_mode {
                    chorder
                    .handle_key_event(handle.clone(), &key_event)?;
            }

            chorder.ensure_inactive()?;
        }
    }

    Ok(())
}
