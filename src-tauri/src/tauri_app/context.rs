use crate::{
    input::{KeyEvent, KeyEventState},
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use device_query::DeviceState;
use std::sync::Arc;
use std::sync::atomic::Ordering;


pub struct AppContext {
    pub device_state: Option<DeviceState>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
}

impl AppContext {
    pub fn new() -> Result<Self> {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };

        let app_mode_state_machine = Arc::new(AppModeStateMachine::new(device_state.clone()));

        Ok(Self {
            device_state,
            key_event_state: KeyEventState::new(app_mode_state_machine.clone()),
            app_mode_state_machine,
        })
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

