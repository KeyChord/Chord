use crate::app::permissions::AppPermissions;

pub(crate) const APP_STATE_STORE_PATH: &str = "app-state.json";

pub fn is_autostart_launch() -> bool {
    std::env::args_os().any(|arg| arg == "--autostart")
}

/// Whether launching should open the settings window. Opening Chord always does, except when
/// Start at Login opened it. A chord run from the command line isn't someone opening Chord, so
/// it only does when they have yet to finish setup or grant its permission.
pub fn should_open_window_on_launch(
    has_startup_command: bool,
    is_onboarding_complete: bool,
) -> bool {
    !is_autostart_launch()
        && (!has_startup_command
            || !is_onboarding_complete
            || !AppPermissions::check_now().accessibility)
}
