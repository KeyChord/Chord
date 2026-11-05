//! Where Chord is running from. Someone who double-clicks Chord inside its disk image gets a
//! working app that disappears once the image is ejected, and that can never update or start
//! at login. macOS even runs it from a random read-only mirror ("App Translocation") when the
//! image was downloaded. Offering to move it is friendlier than a warning about any of that.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::AppHandle;

/// The running `.app`, or `None` for a bare executable such as `cargo run`.
fn current_bundle() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    bundle_of(&executable).map(Path::to_path_buf)
}

fn bundle_of(executable: &Path) -> Option<&Path> {
    executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
}

/// True for a path inside a disk image, or a translocated copy of one.
fn is_disposable_location(bundle: &Path) -> bool {
    let path = bundle.to_string_lossy();
    path.starts_with("/Volumes/") || path.contains("/AppTranslocation/")
}

pub fn needs_move_to_applications() -> bool {
    current_bundle().is_some_and(|bundle| is_disposable_location(&bundle))
}

/// Ejects the disk image Chord was installed from, once it's running from somewhere else. Its
/// window would otherwise sit on top of setup, and people tend to leave installers mounted and
/// later open the wrong copy. A disk image in use stays mounted, as this doesn't force it.
pub fn eject_installer() {
    let Some(bundle) = current_bundle() else {
        return;
    };
    if is_disposable_location(&bundle) {
        return;
    }
    let (Some(name), Some(stem)) = (bundle.file_name(), bundle.file_stem()) else {
        return;
    };
    let volume = Path::new("/Volumes").join(stem);
    if !volume.join(name).exists() {
        return;
    }
    std::thread::spawn(move || {
        if let Err(error) = run(Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet"])
            .arg(&volume))
        {
            log::info!("Left the installer mounted: {error:#}");
        }
    });
}

/// macOS 27 renamed the Accessibility page under Privacy & Security.
pub fn accessibility_pane_name() -> &'static str {
    pane_name_for_major_version(macos_major_version())
}

fn pane_name_for_major_version(major: isize) -> &'static str {
    if major >= 27 {
        "Device Control and Data Access"
    } else {
        "Accessibility"
    }
}

#[cfg(target_os = "macos")]
fn macos_major_version() -> isize {
    objc2_foundation::NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion
}

#[cfg(not(target_os = "macos"))]
fn macos_major_version() -> isize {
    0
}

/// The disk image's mount point, for ejecting it once nothing runs from it. A translocated app's
/// path says nothing about where it came from, so only macOS can answer for those.
#[cfg(target_os = "macos")]
fn source_volume(bundle: &Path) -> Option<PathBuf> {
    use objc2_core_foundation::{CFRetained, CFURL};
    use std::ffi::c_void;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecTranslocateCreateOriginalPathForURL(
            translocated: &CFURL,
            error: *mut *mut c_void,
        ) -> *mut CFURL;
    }

    let original = if bundle.starts_with("/Volumes") {
        bundle.to_path_buf()
    } else {
        let url = CFURL::from_file_path(bundle)?;
        // SAFETY: `url` is a valid CFURL, and the Create rule gives us the returned reference.
        let original = unsafe {
            let raw = SecTranslocateCreateOriginalPathForURL(&url, std::ptr::null_mut());
            CFRetained::from_raw(std::ptr::NonNull::new(raw)?)
        };
        original.to_file_path()?
    };
    let mut components = original.components();
    let volume: PathBuf = components.by_ref().take(3).collect();
    volume.starts_with("/Volumes").then_some(volume)
}

#[cfg(not(target_os = "macos"))]
fn source_volume(_bundle: &Path) -> Option<PathBuf> {
    None
}

fn run(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("Could not run {command:?}"))?;
    anyhow::ensure!(
        output.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

/// Copies the app into place in one step, replacing an older copy only once the new one is whole.
fn install_copy(bundle: &Path, destination: &Path) -> Result<()> {
    let staging = destination.with_extension("app.installing");
    let _ = std::fs::remove_dir_all(&staging);
    run(Command::new("/usr/bin/ditto").arg(bundle).arg(&staging))
        .context("Could not copy Chord to the Applications folder")?;
    // The person already approved this app when they opened it from the disk image.
    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&staging)
        .status();
    if destination.exists() {
        std::fs::remove_dir_all(destination)
            .with_context(|| format!("Could not replace {}", destination.display()))?;
    }
    std::fs::rename(&staging, destination)
        .with_context(|| format!("Could not move Chord to {}", destination.display()))
}

/// Installs the running app in /Applications, quits, and opens the installed copy, ejecting
/// the disk image in between. `caffeinate -w` waits for this process without polling.
pub fn move_to_applications_and_relaunch(handle: &AppHandle) -> Result<()> {
    let bundle = current_bundle().context("Chord is not running from an app bundle")?;
    let name = bundle.file_name().context("The app has no name")?;
    let destination = Path::new("/Applications").join(name);
    install_copy(&bundle, &destination)?;

    let eject = source_volume(&bundle).map_or(String::new(), |volume| {
        format!("/usr/bin/hdiutil detach -quiet {}; ", shell_quote(&volume))
    });
    let script = format!(
        "/usr/bin/caffeinate -w {pid}; {eject}/usr/bin/open {destination}",
        pid = std::process::id(),
        destination = shell_quote(&destination),
    );
    Command::new("/bin/sh")
        .args(["-c", &script])
        .spawn()
        .context("Could not schedule reopening Chord")?;
    handle.exit(0);
    Ok(())
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_bundle_of_the_main_executable() {
        assert_eq!(
            bundle_of(Path::new("/Volumes/Chord/Chord.app/Contents/MacOS/chord")),
            Some(Path::new("/Volumes/Chord/Chord.app"))
        );
        assert_eq!(bundle_of(Path::new("/usr/local/bin/chord")), None);
    }

    #[test]
    fn disk_images_and_translocation_are_disposable() {
        assert!(is_disposable_location(Path::new(
            "/Volumes/Chord/Chord.app"
        )));
        assert!(is_disposable_location(Path::new(
            "/private/var/folders/yt/T/AppTranslocation/856B/d/Chord.app"
        )));
        assert!(!is_disposable_location(Path::new(
            "/Applications/Chord.app"
        )));
        assert!(!is_disposable_location(Path::new(
            "/Users/someone/Applications/Chord.app"
        )));
    }

    #[test]
    fn the_settings_page_name_follows_the_macos_version() {
        assert_eq!(pane_name_for_major_version(26), "Accessibility");
        assert_eq!(
            pane_name_for_major_version(27),
            "Device Control and Data Access"
        );
    }

    #[test]
    fn quotes_paths_for_the_shell() {
        assert_eq!(
            shell_quote(Path::new("/Volumes/My 'Disk")),
            r"'/Volumes/My '\''Disk'"
        );
    }
}
