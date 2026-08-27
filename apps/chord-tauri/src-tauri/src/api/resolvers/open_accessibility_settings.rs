use crate::api::ApiImpl;
use crate::app::permissions::{open_system_settings, request_accessibility_prompt_sync};

pub async fn open_accessibility_settings(api: ApiImpl) {
    if let Ok(handle) = api.handle() {
        if let Err(e) = request_accessibility_prompt_sync(&handle) {
            log::error!("Accessibility prompt failed: {e:#}");
        }
        open_system_settings(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            "accessibility",
        );
    }
}
