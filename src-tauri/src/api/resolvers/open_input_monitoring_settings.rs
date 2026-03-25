use crate::api::ApiImpl;
use crate::app::permissions::open_system_settings;

pub async fn open_input_monitoring_settings(api: ApiImpl) {
    let _ = api;
    open_system_settings(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
        "input monitoring",
    );
}
