use crate::api::{ApiImpl, open_system_settings};

pub async fn open_accessibility_settings(api: ApiImpl) {
    let _ = api;
    open_system_settings(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        "accessibility",
    );
}
