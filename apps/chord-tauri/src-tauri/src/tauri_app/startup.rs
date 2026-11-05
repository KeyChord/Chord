use crate::app::permissions::AppPermissions;

pub(crate) const APP_STATE_STORE_PATH: &str = "app-state.json";

pub fn is_autostart_launch() -> bool {
    std::env::args_os().any(|arg| arg == "--autostart")
}

/// Whether launching should open a window: Chord is otherwise a background app, but someone who
/// has yet to finish setup or grant its permission needs to be told what to do next.
pub fn should_open_window_on_launch(is_onboarding_complete: bool) -> bool {
    !is_autostart_launch() && (!is_onboarding_complete || !AppPermissions::check_now().accessibility)
}
