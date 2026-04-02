use crate::app::AppHandleExt;
use crate::input::handle_key_event;
use crate::input::{Key, KeyEvent};
use anyhow::Result;
use keycode::KeyMappingCode;
use std::os::raw::c_int;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::{OnceLock, mpsc::Sender};
use tauri::AppHandle;

static TX: OnceLock<Sender<bool>> = OnceLock::new();

unsafe extern "C" {
    fn start_caps_lock_listener(cb: extern "C" fn(c_int));
    fn toggle_caps() -> c_int;
}

extern "C" fn caps_lock_changed(pressed: c_int) {
    log::debug!("caps_lock_changed: {}", pressed);
    if let Some(tx) = TX.get() {
        if let Err(e) = tx.send(pressed != 0) {
            log::error!("Failed to send caps lock changed event: {e}");
        }
    } else {
        log::error!("No tx found");
    }
}

pub fn register_caps_lock_input_handler(handle: AppHandle) -> Result<()> {
    log::info!("Registering caps lock handler");
    let (tx, rx) = channel();

    TX.set(tx)
        .map_err(|_| anyhow::anyhow!("failed to set tx"))?;

    std::thread::spawn(|| unsafe {
        start_caps_lock_listener(caps_lock_changed);
    });

    let handle = handle.clone();
    std::thread::spawn(move || {
        while let Ok(pressed) = rx.recv() {
            let context = handle.app_context();
            if pressed {
                context
                    .key_event_state
                    .process_event(&KeyEvent::Press(Key(KeyMappingCode::CapsLock)));

                if let Err(e) = handle_key_event(
                    handle.clone(),
                    KeyEvent::Press(Key(KeyMappingCode::CapsLock)),
                ) {
                    log::error!("Failed to handle Caps Lock Press: {e}");
                }
            } else {
                context
                    .key_event_state
                    .process_event(&KeyEvent::Release(Key(KeyMappingCode::CapsLock)));

                if let Err(e) = handle_key_event(
                    handle.clone(),
                    KeyEvent::Release(Key(KeyMappingCode::CapsLock)),
                ) {
                    log::error!("Failed to handle Caps Lock Release: {e}");
                }
            }
        }
    });

    Ok(())
}

pub fn emit_caps_lock() -> Result<()> {
    let rc = unsafe { toggle_caps() };
    if rc == 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "failed to toggle caps lock state via native layer: {rc}"
        ))
    }
}
