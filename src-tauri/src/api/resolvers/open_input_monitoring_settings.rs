use crate::api::ApiImpl;
use crate::app::permissions::{open_system_settings, register_input_monitoring_sync};

pub async fn open_input_monitoring_settings(api: ApiImpl) {
    if let Ok(handle) = api.handle() {
        if let Err(e) = register_input_monitoring_sync(&handle) {
            log::error!("Input monitoring registration failed: {e:#}");
        }
        open_system_settings(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
            "input monitoring",
        );
    }
}
