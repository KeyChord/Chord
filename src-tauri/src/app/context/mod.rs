use crate::app::state::StateSingleton;
use crate::{
    input::{KeyEvent, KeyEventState},
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use device_query::DeviceState;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::AppHandle;

pub struct AppContext {
    pub device_state: Option<DeviceState>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
    _handle: AppHandle,
}

impl StateSingleton for AppContext {
    fn new(handle: AppHandle) -> Self {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };
        let app_mode_state_machine = Arc::new(AppModeStateMachine::new(device_state.clone()));
        let key_event_state = KeyEventState::new(app_mode_state_machine.clone());

        Self {
            _handle: handle,
            key_event_state,
            app_mode_state_machine,
            device_state,
        }
    }
}

impl AppContext {
    pub fn init(&self) -> Result<()> {
        Ok(())
    }

    pub fn get_app_mode(&self) -> AppMode {
        self.app_mode_state_machine.get_app_mode()
    }

    pub fn is_shift_pressed(&self) -> bool {
        self.app_mode_state_machine
            .is_shift_pressed
            .load(Ordering::SeqCst)
    }

    pub fn take_caps_lock_passthrough_on_release(&self, event: &KeyEvent) -> bool {
        self.app_mode_state_machine
            .take_caps_lock_passthrough_on_release(event)
    }
}
