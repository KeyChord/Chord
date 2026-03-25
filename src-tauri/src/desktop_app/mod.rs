mod manager;
pub use manager::*;

use std::time::{Duration, Instant};
use anyhow::Result;
use base64::Engine;
use objc2::__framework_prelude::AnyObject;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication, NSWorkspace, NSWorkspaceLaunchOptions};
use objc2_foundation::{NSDictionary, NSSize, NSString};

pub struct DesktopApp {
    pub bundle_id: String
}

#[derive(Debug)]
#[taurpc::ipc_type]
#[serde(rename_all = "camelCase")]
#[specta(rename_all = "camelCase")]
pub struct DesktopAppMetadata {
    pub bundle_id: String,
    pub display_name: Option<String>,
    pub icon_data_url: Option<String>,
}

impl DesktopApp {
    pub fn new(bundle_id: &str) -> Result<Self> {
        let bundle_id = bundle_id.trim();
        if bundle_id.is_empty() {
            anyhow::bail!("bundle id cannot be empty");
        }

        Ok(Self {
            bundle_id: bundle_id.to_string()
        })
    }


    pub fn get_metadata(&self) -> Result<DesktopAppMetadata> {
        Ok(DesktopAppMetadata {
            bundle_id: self.bundle_id.clone(),
            display_name: self.resolve_display_name(),
            icon_data_url: self.resolve_icon_data_url(),
        })
    }

    pub fn resolve_display_name(&self) -> Option<String> {
        let bundle_id = NSString::from_str(&self.bundle_id);
        let running_apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);

        if let Some(app) = running_apps.iter().next() {
            if let Some(name) = app.localizedName() {
                return Some(name.to_string());
            }
        }

        let workspace = NSWorkspace::sharedWorkspace();
        let app_url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)?;
        let app_name = app_url.lastPathComponent()?;
        let app_name = app_name.to_string();
        Some(
            app_name
                .strip_suffix(".app")
                .unwrap_or(&app_name)
                .to_string(),
        )
    }

    pub fn resolve_path(&self) -> Option<String> {
        let bundle_id = NSString::from_str(&self.bundle_id);
        let workspace = NSWorkspace::sharedWorkspace();
        let app_url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)?;
        let app_path = app_url.path()?;
        Some(app_path.to_string())
    }

    pub fn resolve_icon_data_url(&self) -> Option<String> {
        let app_path = self.resolve_path()?;
        let workspace = NSWorkspace::sharedWorkspace();
        let app_path = NSString::from_str(&app_path);
        let icon = workspace.iconForFile(&app_path);
        icon.setSize(NSSize::new(20.0, 20.0));

        let tiff = icon.TIFFRepresentation()?;
        let bitmap = NSBitmapImageRep::imageRepWithData(&tiff)?;
        let properties = NSDictionary::<objc2_app_kit::NSBitmapImageRepPropertyKey, AnyObject>::new();
        let png_data = unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        }?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(png_data.to_vec());
        Some(format!("data:image/png;base64,{encoded}"))
    }
}
