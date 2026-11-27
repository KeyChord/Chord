//! Media keys in shortcuts: brightness, volume and playback. macOS takes these as the
//! system-defined events the keyboard's own media keys send, not as virtual key codes, so a
//! shortcut such as `shortcut = 'BrightnessDown'` cannot be pressed like an ordinary key.

use keycode::KeyMappingCode;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSEventModifierFlags;

#[cfg(target_os = "macos")]
pub fn modifier_flag(code: KeyMappingCode) -> Option<NSEventModifierFlags> {
    Some(match code {
        KeyMappingCode::ShiftLeft | KeyMappingCode::ShiftRight => NSEventModifierFlags::Shift,
        KeyMappingCode::ControlLeft | KeyMappingCode::ControlRight => NSEventModifierFlags::Control,
        KeyMappingCode::AltLeft | KeyMappingCode::AltRight => NSEventModifierFlags::Option,
        KeyMappingCode::MetaLeft | KeyMappingCode::MetaRight => NSEventModifierFlags::Command,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKey {
    BrightnessUp,
    BrightnessDown,
    VolumeUp,
    VolumeDown,
    Mute,
    PlayPause,
    Next,
    Previous,
    Eject,
}

impl MediaKey {
    pub fn from_code(code: KeyMappingCode) -> Option<Self> {
        Some(match code {
            KeyMappingCode::BrightnessUp => Self::BrightnessUp,
            KeyMappingCode::BrightnessDown => Self::BrightnessDown,
            KeyMappingCode::AudioVolumeUp => Self::VolumeUp,
            KeyMappingCode::AudioVolumeDown => Self::VolumeDown,
            KeyMappingCode::AudioVolumeMute => Self::Mute,
            KeyMappingCode::MediaPlayPause => Self::PlayPause,
            KeyMappingCode::MediaTrackNext => Self::Next,
            KeyMappingCode::MediaTrackPrevious => Self::Previous,
            KeyMappingCode::Eject => Self::Eject,
            _ => return None,
        })
    }

    /// The key's `NX_KEYTYPE_*` value from IOKit's `ev_keymap.h`.
    fn nx_key_type(self) -> isize {
        match self {
            Self::VolumeUp => 0,
            Self::VolumeDown => 1,
            Self::BrightnessUp => 2,
            Self::BrightnessDown => 3,
            Self::Mute => 7,
            Self::Eject => 14,
            Self::PlayPause => 16,
            Self::Next => 17,
            Self::Previous => 18,
        }
    }
}

/// Presses and releases `key`. Runs on the main thread.
#[cfg(target_os = "macos")]
pub fn press(
    key: MediaKey,
    simulated_modifiers: NSEventModifierFlags,
    suppress_shift: bool,
) -> anyhow::Result<()> {
    use objc2_core_graphics::{CGEventSource, CGEventSourceStateID};

    let physical = CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState);
    let modifiers = effective_modifiers(physical.bits(), simulated_modifiers, suppress_shift);
    // Many Macs, Apple silicon ones especially, accept a synthesized brightness key and
    // change nothing. Plain presses can set brightness directly. Modified presses are
    // system shortcuts, such as Option+Brightness opening Displays settings.
    if modifiers.is_empty()
        && matches!(key, MediaKey::BrightnessUp | MediaKey::BrightnessDown)
        && brightness::step(key == MediaKey::BrightnessUp)
    {
        return Ok(());
    }
    post_system_defined(key.nx_key_type(), modifiers)
}

#[cfg(target_os = "macos")]
fn effective_modifiers(
    physical: u64,
    simulated: NSEventModifierFlags,
    suppress_shift: bool,
) -> NSEventModifierFlags {
    let shortcut_modifiers = NSEventModifierFlags::Shift
        | NSEventModifierFlags::Control
        | NSEventModifierFlags::Option
        | NSEventModifierFlags::Command
        | NSEventModifierFlags::Function;
    let mut flags = (NSEventModifierFlags(physical as usize) | simulated) & shortcut_modifiers;
    if suppress_shift {
        flags.remove(NSEventModifierFlags::Shift);
    }
    flags
}

#[cfg(target_os = "macos")]
fn post_system_defined(nx_key_type: isize, modifiers: NSEventModifierFlags) -> anyhow::Result<()> {
    use anyhow::Context;
    use objc2_app_kit::{NSEvent, NSEventType};
    use objc2_core_graphics::{CGEvent, CGEventTapLocation};
    use objc2_foundation::NSPoint;

    /// `NX_SUBTYPE_AUX_CONTROL_BUTTONS`: media keys.
    const AUX_CONTROL_BUTTONS: i16 = 8;
    // `NX_KEYDOWN` and `NX_KEYUP`. A real press carries the state in the flags as well.
    for state in [0x0a_isize, 0x0b] {
        let event =
            NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                NSEventType::SystemDefined,
                NSPoint::new(0.0, 0.0),
                modifiers | NSEventModifierFlags((state as usize) << 8),
                0.0,
                0,
                None,
                AUX_CONTROL_BUTTONS,
                (nx_key_type << 16) | (state << 8),
                -1,
            )
            .context("macOS refused to create the media key event")?;
        let event = event
            .CGEvent()
            .context("macOS refused to convert the media key event")?;
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }
    Ok(())
}

/// DisplayServices, the framework behind Control Center's brightness slider, for the
/// built-in display and Apple displays.
#[cfg(target_os = "macos")]
mod brightness {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    type GetBrightness = unsafe extern "C" fn(u32, *mut f32) -> i32;
    type SetBrightness = unsafe extern "C" fn(u32, f32) -> i32;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGGetOnlineDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    }

    /// The brightness keys move in sixteen steps from off to full.
    const STEPS: f32 = 16.0;

    fn services() -> Option<(GetBrightness, SetBrightness)> {
        static SERVICES: OnceLock<Option<(usize, usize)>> = OnceLock::new();
        let (get, set) = (*SERVICES.get_or_init(|| {
            // SAFETY: dlopen and dlsym with static C strings; a missing symbol is skipped.
            unsafe {
                let handle = libc::dlopen(
                    c"/System/Library/PrivateFrameworks/DisplayServices.framework/DisplayServices"
                        .as_ptr(),
                    libc::RTLD_LAZY,
                );
                if handle.is_null() {
                    return None;
                }
                let get = libc::dlsym(handle, c"DisplayServicesGetBrightness".as_ptr());
                let set = libc::dlsym(handle, c"DisplayServicesSetBrightness".as_ptr());
                (!get.is_null() && !set.is_null()).then_some((get as usize, set as usize))
            }
        }))?;
        // SAFETY: the symbols have these C signatures in every macOS that ships them.
        unsafe {
            Some((
                std::mem::transmute::<*mut c_void, GetBrightness>(get as *mut c_void),
                std::mem::transmute::<*mut c_void, SetBrightness>(set as *mut c_void),
            ))
        }
    }

    /// Moves the brightness one key step on the main display, or else the first display that
    /// supports it. Returns false when no display's brightness can be set this way.
    pub fn step(up: bool) -> bool {
        let Some((get, set)) = services() else {
            return false;
        };
        // SAFETY: the out parameters are live storage of the sizes CoreGraphics expects.
        let displays = unsafe {
            let mut count = 0u32;
            if CGGetOnlineDisplayList(0, std::ptr::null_mut(), &mut count) != 0 || count == 0 {
                return false;
            }
            let mut displays = vec![0u32; count as usize];
            if CGGetOnlineDisplayList(count, displays.as_mut_ptr(), &mut count) != 0 {
                return false;
            }
            displays.truncate(count as usize);
            displays
        };
        // SAFETY: a plain query.
        let main = unsafe { CGMainDisplayID() };
        let ordered = std::iter::once(main).chain(displays.into_iter().filter(|&d| d != main));
        for display in ordered {
            let mut current = 0.0f32;
            // SAFETY: `current` is live storage; DisplayServices returns 0 on success.
            if unsafe { get(display, &mut current) } != 0 {
                continue;
            }
            let next =
                (((current * STEPS).round() + if up { 1.0 } else { -1.0 }) / STEPS).clamp(0.0, 1.0);
            // SAFETY: as above.
            if unsafe { set(display, next) } == 0 {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_keys_map_to_their_macos_key_types() {
        assert_eq!(
            MediaKey::from_code(KeyMappingCode::BrightnessDown).map(MediaKey::nx_key_type),
            Some(3)
        );
        assert_eq!(
            MediaKey::from_code(KeyMappingCode::AudioVolumeUp).map(MediaKey::nx_key_type),
            Some(0)
        );
        assert_eq!(
            MediaKey::from_code(KeyMappingCode::MediaPlayPause).map(MediaKey::nx_key_type),
            Some(16)
        );
        assert_eq!(MediaKey::from_code(KeyMappingCode::KeyA), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn media_keys_combine_simulated_and_physical_modifiers() {
        let physical = NSEventModifierFlags::CapsLock | NSEventModifierFlags::Shift;
        let simulated = NSEventModifierFlags::Option;
        assert_eq!(
            effective_modifiers(physical.bits() as u64, simulated, false),
            simulated | NSEventModifierFlags::Shift
        );
        assert_eq!(
            effective_modifiers(physical.bits() as u64, simulated, true),
            simulated
        );
    }
}
