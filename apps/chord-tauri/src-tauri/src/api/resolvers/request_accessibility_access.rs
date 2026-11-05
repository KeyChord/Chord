use crate::api::ApiImpl;
use crate::app::permissions::request_accessibility_prompt_sync;

/// Shows only macOS's own prompt, whose button then opens the right page of System Settings.
/// Opening the page as well would stack the prompt on top of the page it is asking you to open.
pub async fn request_accessibility_access(api: ApiImpl) {
    if let Ok(handle) = api.handle() {
        if let Err(error) = request_accessibility_prompt_sync(&handle) {
            log::error!("Accessibility prompt failed: {error:#}");
        }
    }
}
