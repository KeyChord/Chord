use crate::app::permissions::AppPermissions;

pub(crate) const APP_STATE_STORE_PATH: &str = "app-state.json";

pub fn is_autostart_launch() -> bool {
    std::env::args_os().any(|arg| arg == "--autostart")
}

pub fn should_show_permission_dialog() -> bool {
    !is_autostart_launch() && !AppPermissions::check_now().accessibility
}
