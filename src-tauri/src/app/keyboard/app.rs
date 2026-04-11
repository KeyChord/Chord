use std::sync::atomic::AtomicU16;
use tauri::AppHandle;
use crate::app::AppSingleton;
use crate::app::keyboard::AppKeyboard;
use device_query::DeviceState;

impl AppSingleton<()> for AppKeyboard {
    fn new(handle: AppHandle) -> Self {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };

        Self {
            device_state,
            modifier_flags: AtomicU16::new(0),
            handle,
        }
    }

    fn init(&self, _: ()) -> anyhow::Result<()> {
        self.register_input_handler();

        Ok(())
    }

}

