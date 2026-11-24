use crate::define_observable;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontmostState {
    pub frontmost_app_bundle_id: Option<String>,
    pub overlay_app_bundle_id: Option<String>,
    pub file_picker_active: bool,
}

impl FrontmostState {
    pub fn chord_application(&self) -> Option<&str> {
        self.overlay_app_bundle_id
            .as_deref()
            .or(self.frontmost_app_bundle_id.as_deref())
    }

    pub fn chord_file_picker_active(&self) -> bool {
        self.file_picker_active && self.overlay_app_bundle_id.is_none()
    }
}

define_observable!(
    #[derive(Debug)]
    pub struct FrontmostObservable(FrontmostState);
    id: "frontmost";
);
