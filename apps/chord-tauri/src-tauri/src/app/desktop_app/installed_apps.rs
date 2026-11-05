//! The apps someone could open with their first chord, those in their Dock first.
use crate::api::InstalledApp;
use base64::Engine;
use objc2::__framework_prelude::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Drawn at twice the size shown, for Retina displays.
const ICON_PIXELS: f64 = 80.0;
const FINDER: &str = "/System/Library/CoreServices/Finder.app";

struct AppBundle {
    path: PathBuf,
    bundle_id: String,
    name: String,
}

fn read_bundle(path: &Path) -> Option<AppBundle> {
    let info = plist::Value::from_file(path.join("Contents/Info.plist")).ok()?;
    let info = info.as_dictionary()?;
    // Agents and helpers have no window to bring forward.
    let is_background = ["LSUIElement", "LSBackgroundOnly"].iter().any(|key| {
        info.get(key)
            .is_some_and(|value| value.as_boolean() == Some(true) || value.as_string() == Some("1"))
    });
    if is_background {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let name = info
        .get("CFBundleDisplayName")
        .or_else(|| info.get("CFBundleName"))
        .and_then(plist::Value::as_string)
        .filter(|name| !name.trim().is_empty())
        .map_or(stem, str::to_owned);
    Some(AppBundle {
        path: path.to_path_buf(),
        bundle_id: info.get("CFBundleIdentifier")?.as_string()?.to_owned(),
        name,
    })
}

fn apps_in(directory: &Path) -> impl Iterator<Item = AppBundle> {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .filter_map(|path| read_bundle(&path))
}

/// Bundle IDs of the apps kept in the Dock, in Dock order.
fn dock_bundle_ids(home: &Path) -> Vec<String> {
    let preferences = home.join("Library/Preferences/com.apple.dock.plist");
    let Ok(dock) = plist::Value::from_file(preferences) else {
        return Vec::new();
    };
    let tiles = dock
        .as_dictionary()
        .and_then(|dock| dock.get("persistent-apps"))
        .and_then(plist::Value::as_array);
    tiles
        .into_iter()
        .flatten()
        .filter_map(|tile| {
            tile.as_dictionary()?
                .get("tile-data")?
                .as_dictionary()?
                .get("bundle-identifier")?
                .as_string()
                .map(str::to_owned)
        })
        .collect()
}

/// The app's icon, drawn only at the size shown. `TIFFRepresentation` would rasterize every size
/// the icon comes in, up to 1024 px, which takes a noticeable moment for each app.
fn icon_data_url(path: &Path) -> Option<String> {
    let path = NSString::from_str(&path.to_string_lossy());
    let icon = NSWorkspace::sharedWorkspace().iconForFile(&path);
    let size = NSSize::new(ICON_PIXELS, ICON_PIXELS);
    icon.setSize(size);
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    let image = unsafe { icon.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
    let bitmap = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &image);
    let properties = NSDictionary::<objc2_app_kit::NSBitmapImageRepPropertyKey, AnyObject>::new();
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
    }?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png.to_vec());
    Some(format!("data:image/png;base64,{encoded}"))
}

/// Everything in the Applications folders, with Dock apps first and then by name.
pub fn installed_apps() -> Vec<InstalledApp> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let directories = [
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
        home.join("Applications"),
    ];
    let this_app = tauri_bundle_id();
    let mut seen = HashSet::new();
    let mut apps: Vec<AppBundle> = read_bundle(Path::new(FINDER))
        .into_iter()
        .chain(directories.iter().flat_map(|directory| apps_in(directory)))
        .filter(|app| Some(app.bundle_id.as_str()) != this_app.as_deref())
        .filter(|app| seen.insert(app.bundle_id.clone()))
        .collect();

    let dock = dock_bundle_ids(&home);
    let rank = |app: &AppBundle| {
        // The Finder is always first in the Dock but isn't listed in its preferences.
        if app.path == Path::new(FINDER) {
            return Some(0);
        }
        dock.iter()
            .position(|bundle_id| bundle_id == &app.bundle_id)
            .map(|position| position + 1)
    };
    apps.sort_by_cached_key(|app| (rank(app).unwrap_or(usize::MAX), app.name.to_lowercase()));
    apps.into_iter()
        .map(|app| InstalledApp {
            in_dock: rank(&app).is_some(),
            icon_data_url: icon_data_url(&app.path),
            bundle_id: app.bundle_id,
            name: app.name,
        })
        .collect()
}

fn tauri_bundle_id() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    let bundle = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))?;
    Some(read_bundle(bundle)?.bundle_id)
}
