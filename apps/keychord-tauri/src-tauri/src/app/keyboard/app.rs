use crate::app::AppSingleton;
use crate::app::keyboard::AppKeyboard;
use anyhow::Result;
use device_query::DeviceState;
use nject::provider;
use std::sync::atomic::AtomicU16;
use tauri::AppHandle;
use crate::state::KeyboardObservable;

#[provider]
pub struct AppKeyboardProvider {
    #[provide(KeyboardObservable, |v| v.provide())]
    pub keyboard_observable: KeyboardObservable,
    
    #[provide(AppHandle, |h| h.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for AppKeyboard {
    fn init(&self) -> Result<()> {
        self.register_input_handler();

        Ok(())
    }
}
