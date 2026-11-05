use crate::api::{ApiImpl, AppResult, OnboardingInfo};
use crate::app::settings::install_location;

pub async fn get_onboarding_info(_api: ApiImpl) -> AppResult<OnboardingInfo> {
    Ok(OnboardingInfo {
        needs_move_to_applications: install_location::needs_move_to_applications(),
        accessibility_pane_name: install_location::accessibility_pane_name().to_owned(),
    })
}
