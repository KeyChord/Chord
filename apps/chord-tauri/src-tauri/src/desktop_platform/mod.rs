use crate::models::{Key, KeyEvent, SimulatedShortcutAction};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DesktopProcess {
    pub pid: u32,
    pub executable: String,
    pub window_class: Option<String>,
}

pub(crate) fn simulate_keys(actions: &[SimulatedShortcutAction]) -> Result<()> {
    if actions.is_empty() {
        return Ok(());
    }
    let suppress_shift = actions.iter().any(|action| match action {
        SimulatedShortcutAction::Press(_, suppress)
        | SimulatedShortcutAction::Release(_, suppress) => *suppress,
    });
    let held = if suppress_shift {
        held_modifier_keys()
            .into_iter()
            .filter(|key| {
                matches!(
                    key.0,
                    keycode::KeyMappingCode::ShiftLeft | keycode::KeyMappingCode::ShiftRight
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut released = Vec::new();
    for key in held {
        if let Err(error) = inject_key(key, false) {
            for released in released {
                let _ = inject_key(released, true);
            }
            return Err(error);
        }
        released.push(key);
    }
    let result = actions.iter().try_for_each(|action| match *action {
        SimulatedShortcutAction::Press(key, _) => inject_key(key, true),
        SimulatedShortcutAction::Release(key, _) => inject_key(key, false),
    });
    released
        .into_iter()
        .fold(result, |result, key| result.and(inject_key(key, true)))
}

pub(crate) type KeyFilter = dyn FnMut(KeyEvent) -> bool + 'static;
pub(crate) type Ready = dyn Fn() + 'static;

fn running(stop: &AtomicBool) -> bool {
    !stop.load(std::sync::atomic::Ordering::Acquire)
}

pub(crate) fn installed_app(id: String, name: String) -> crate::api::InstalledApp {
    crate::api::InstalledApp {
        bundle_id: id,
        name,
        icon_data_url: None,
        in_dock: false,
    }
}

pub(crate) fn executable_id(name: &str) -> String {
    // An application ID maps back to path components. Escape punctuation that
    // would introduce extra components or an invalid Windows filename.
    let name = name.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    let mut id = String::from("executable.");
    for byte in name.bytes() {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' | b'-' => id.push(char::from(byte)),
            _ => {
                use std::fmt::Write;
                let _ = write!(id, "_{byte:02x}");
            }
        }
    }
    id
}

pub(crate) fn executable_name(path: PathBuf) -> Option<String> {
    Some(path.file_name()?.to_string_lossy().into_owned())
}
