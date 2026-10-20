//! Identifies running apps, including bare executables that macOS reports without a bundle
//! identifier (for example LiveSplit One's release build, which ships without an `.app` bundle).
//!
//! An app's ID is, in order:
//! 1. its bundle identifier;
//! 2. for a bare executable, a developer-declared ID: a `CFBundleIdentifier` embedded in its
//!    `__TEXT,__info_plist` section, else the identifier of a certificate (non-ad-hoc) signature;
//! 3. the app ID of a chords file that declares the executable's file name in `executables`.
//!
//! Ad-hoc signing identifiers are ignored: the linker derives them from the link-time file name,
//! so Cargo builds carry a per-build hash (`name-<hash>`), `codesign -s -` appends the binary's
//! UUID, and every Go binary is `a.out`.

use crate::app::AppHandleExt;
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use objc2::rc::Retained;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_core_foundation::{
    CFBundle, CFDictionary, CFNumber, CFRetained, CFString, CFType, CFURL,
};
use objc2_foundation::NSString;
use objc2_security::{
    SecCSFlags, SecCode, SecCodeSignatureFlags, SecStaticCode, kSecCSSigningInformation,
    kSecCodeInfoFlags, kSecCodeInfoIdentifier,
};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppIdentity {
    /// The bundle identifier, or a bare executable's developer-declared ID.
    pub declared_id: Option<String>,
    /// File name of a bare executable without a declared ID, which chords files can claim with
    /// `executables`.
    pub executable_name: Option<String>,
}

impl AppIdentity {
    pub fn of(app: &NSRunningApplication) -> Self {
        if let Some(bundle_id) = app.bundleIdentifier() {
            return Self {
                declared_id: Some(bundle_id.to_string()),
                executable_name: None,
            };
        }
        let Some(path) = app
            .executableURL()
            .and_then(|url| url.path())
            .map(|path| PathBuf::from(path.to_string()))
        else {
            return Self::default();
        };
        match declared_executable_id(&path) {
            Some(id) => Self {
                declared_id: Some(id),
                executable_name: None,
            },
            None => Self {
                declared_id: None,
                executable_name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
            },
        }
    }

    pub fn from_bundle_id(bundle_id: Option<String>) -> Self {
        Self {
            declared_id: bundle_id,
            executable_name: None,
        }
    }

    /// The app ID used for chords files, handler context and lifecycle callbacks.
    pub fn app_id(&self, app_id_for_executable: impl Fn(&str) -> Option<String>) -> Option<String> {
        self.declared_id
            .clone()
            .or_else(|| app_id_for_executable(self.executable_name.as_deref()?))
    }

    /// Resolves executables through the loaded packages' `executables` declarations.
    pub fn resolve(&self, handle: &AppHandle) -> Option<String> {
        let packages = handle.app_state().chord_package_manager();
        self.app_id(|name| packages.app_id_for_executable(name))
    }
}

/// The running apps with an app ID. Bundle IDs use LaunchServices directly; other IDs compare
/// each bundle-less app's identity.
pub fn running_apps_with_id(
    handle: &AppHandle,
    app_id: &str,
) -> Vec<Retained<NSRunningApplication>> {
    let apps =
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(app_id));
    if !apps.is_empty() {
        return apps.to_vec();
    }
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|app| {
            app.bundleIdentifier().is_none()
                && AppIdentity::of(app).resolve(handle).as_deref() == Some(app_id)
        })
        .collect()
}

/// A developer-declared ID for an executable that macOS reports without a bundle identifier.
pub fn declared_executable_id(path: &Path) -> Option<String> {
    let url = CFURL::from_file_path(path)?;
    embedded_bundle_id(&url).or_else(|| certificate_signing_id(&url))
}

fn dictionary_value<'a>(dict: &'a CFDictionary, key: &CFString) -> Option<&'a CFType> {
    // SAFETY: both dictionaries hold CF objects and stay retained while the reference lives.
    let value = unsafe { dict.value((key as *const CFString).cast::<c_void>()) };
    unsafe { value.cast::<CFType>().as_ref() }
}

fn dictionary_string(dict: &CFDictionary, key: &CFString) -> Option<String> {
    let value = dictionary_value(dict, key)?
        .downcast_ref::<CFString>()?
        .to_string();
    (!value.is_empty()).then_some(value)
}

/// LaunchServices ignores an Info.plist embedded in a bare executable, but CoreFoundation reads it.
fn embedded_bundle_id(url: &CFURL) -> Option<String> {
    let info = CFBundle::info_dictionary_for_url(Some(url))?;
    dictionary_string(&info, &CFString::from_static_str("CFBundleIdentifier"))
}

fn certificate_signing_id(url: &CFURL) -> Option<String> {
    let mut code: *const SecStaticCode = ptr::null();
    // SAFETY: `code` is live storage for the created object, which is returned retained.
    let status =
        unsafe { SecStaticCode::create_with_path(url, SecCSFlags(0), NonNull::from(&mut code)) };
    if status != 0 {
        return None;
    }
    let code = unsafe { CFRetained::from_raw(NonNull::new(code.cast_mut())?) };
    let mut info: *const CFDictionary = ptr::null();
    // SAFETY: as above; the dictionary is returned retained.
    let status = unsafe {
        SecCode::copy_signing_information(
            &code,
            SecCSFlags(kSecCSSigningInformation),
            NonNull::from(&mut info),
        )
    };
    if status != 0 {
        return None;
    }
    let info = unsafe { CFRetained::from_raw(NonNull::new(info.cast_mut())?) };
    // SAFETY: Security framework constants are immutable CF strings.
    let flags = dictionary_value(&info, unsafe { kSecCodeInfoFlags })?
        .downcast_ref::<CFNumber>()?
        .as_i64()?;
    if flags & i64::from(SecCodeSignatureFlags::Adhoc.0) != 0 {
        return None;
    }
    dictionary_string(&info, unsafe { kSecCodeInfoIdentifier })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_info_plist_declares_an_id() {
        assert_eq!(
            declared_executable_id(Path::new("/usr/bin/automator")).as_deref(),
            Some("com.apple.automatorCLI")
        );
    }

    #[test]
    fn certificate_signatures_declare_an_id() {
        // Apple-signed and without an embedded Info.plist.
        assert_eq!(
            declared_executable_id(Path::new("/usr/bin/true")).as_deref(),
            Some("com.apple.true")
        );
    }

    #[test]
    fn ad_hoc_signatures_do_not_declare_an_id() {
        // Cargo links test binaries with an ad-hoc signature named after the hashed file name.
        let test_binary = std::env::current_exe().unwrap();
        assert_eq!(declared_executable_id(&test_binary), None);
        assert_eq!(declared_executable_id(Path::new("/nonexistent/app")), None);
    }

    #[test]
    fn executables_fall_back_to_declared_aliases() {
        let aliases =
            |name: &str| (name == "LiveSplit One").then(|| "org.livesplit.one".to_string());
        let bare = AppIdentity {
            declared_id: None,
            executable_name: Some("LiveSplit One".into()),
        };
        assert_eq!(bare.app_id(aliases).as_deref(), Some("org.livesplit.one"));
        let unknown = AppIdentity {
            declared_id: None,
            executable_name: Some("Other".into()),
        };
        assert_eq!(unknown.app_id(aliases), None);
        // A declared ID always wins over an alias for the same file name.
        let declared = AppIdentity {
            declared_id: Some("com.example.app".into()),
            executable_name: Some("LiveSplit One".into()),
        };
        assert_eq!(declared.app_id(aliases).as_deref(), Some("com.example.app"));
        assert_eq!(AppIdentity::default().app_id(aliases), None);
    }
}
