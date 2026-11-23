use crate::api::ApiImpl;
#[cfg(target_os = "macos")]
use crate::app::keyboard::voice_over::UTILITY_BUNDLE_ID;

pub async fn open_voice_over_utility(_api: ApiImpl) {
    #[cfg(target_os = "macos")]
    if let Err(error) = std::process::Command::new("open")
        .args(["-b", UTILITY_BUNDLE_ID])
        .spawn()
    {
        log::error!("Failed to open VoiceOver Utility: {error}");
    }
}
