//! The keyboard event tap.
//!
//! Chord owns its `CGEventTap` directly instead of going through a library so
//! that the tap can be re-enabled, health-checked, stopped and restarted. A
//! tap that macOS disables (callback timeout, user-input safety cutoff, wake
//! from sleep) is otherwise silently dead: events keep flowing to other apps
//! and nothing tells the process.

use super::caps_remap::REMAPPED_KEYCODE;
use super::caps_space_gap::event_time_ns;
use super::health::InputHealth;
use crate::models::{Key, KeyEvent, KeyEventAction};
use anyhow::{Context, Result, anyhow};
use keycode::KeyMappingCode;
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopRunResult, kCFRunLoopCommonModes,
    kCFRunLoopDefaultMode,
};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// macOS virtual keycode of Caps Lock. A native toggle produces it; changes made
/// through `IOHIDSetModifierLockState` arrive with keycode 255 or the code of the next
/// key, and Chord rewrites them to it (see `TapEvent::CapsLockChanged`).
const CAPS_LOCK_KEYCODE: i64 = 0x39;

/// macOS virtual keycode of Space.
pub(super) const SPACE_KEYCODE: i64 = 0x31;

/// Marker rdev puts on the events Chord itself posts; those must pass through untouched.
pub(super) const SYNTHETIC_EVENT_USER_DATA: [i64; 2] = [0xDEAD_BEEF, 0xDEAD_DEAD];

/// How often the tap thread wakes up to verify the tap is still enabled.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(250);
const START_TIMEOUT: Duration = Duration::from_secs(5);

// Device-dependent modifier bits carried in `CGEventFlags` (`IOLLEvent.h`).
const NX_DEVICELCTLKEYMASK: u64 = 0x0000_0001;
const NX_DEVICELSHIFTKEYMASK: u64 = 0x0000_0002;
const NX_DEVICERSHIFTKEYMASK: u64 = 0x0000_0004;
const NX_DEVICELCMDKEYMASK: u64 = 0x0000_0008;
const NX_DEVICERCMDKEYMASK: u64 = 0x0000_0010;
const NX_DEVICELALTKEYMASK: u64 = 0x0000_0020;
const NX_DEVICERALTKEYMASK: u64 = 0x0000_0040;
const NX_DEVICERCTLKEYMASK: u64 = 0x0000_2000;
// Device-independent modifier bits.
const NX_SHIFTMASK: u64 = 0x0002_0000;
const NX_CONTROLMASK: u64 = 0x0004_0000;
const NX_ALTERNATEMASK: u64 = 0x0008_0000;
const NX_COMMANDMASK: u64 = 0x0010_0000;
const NX_SECONDARYFNMASK: u64 = 0x0080_0000;
const NX_ALPHASHIFTMASK: u64 = 0x0001_0000;

/// Flags that change only when a modifier goes down or up, or Caps Lock toggles.
/// Other bits ride along on ordinary keys: arrow keys carry the numeric-pad and Fn
/// bits, for example.
const HELD_STATE_MASK: u64 = NX_DEVICELCTLKEYMASK
    | NX_DEVICELSHIFTKEYMASK
    | NX_DEVICERSHIFTKEYMASK
    | NX_DEVICELCMDKEYMASK
    | NX_DEVICERCMDKEYMASK
    | NX_DEVICELALTKEYMASK
    | NX_DEVICERALTKEYMASK
    | NX_DEVICERCTLKEYMASK
    | NX_ALPHASHIFTMASK
    | NX_SHIFTMASK
    | NX_CONTROLMASK
    | NX_ALTERNATEMASK
    | NX_COMMANDMASK;

#[derive(Debug, PartialEq)]
pub enum TapEvent {
    Key(KeyEvent),
    /// The remapped Caps key (F20) went down. `caps_lock_on` is the lock state before
    /// the press: with the key remapped, macOS never changes it on its own.
    CapsPressed {
        caps_lock_on: bool,
    },
    /// An auto-repeat of the remapped Caps key (F20). Swallowed without reaching the
    /// controller: Caps never repeats.
    RemappedCapsRepeat,
    /// macOS toggled Caps Lock itself, so Chord's key mapping is gone (another tool
    /// rewrote `UserKeyMapping`). The toggle already happened and reaches apps.
    NativeCapsToggle,
    /// Caps Lock changed through `IOHIDSetModifierLockState`, by Chord or another
    /// app: a flagsChanged event in which nothing but the Caps Lock flag changed.
    /// macOS reports it with keycode 255, or with the code of whichever key comes
    /// next, while apps expect the keycode 57 of a real Caps Lock toggle. Chromium
    /// drops it, and Safari reports an "Unidentified" key.
    CapsLockChanged,
    /// A flagsChanged event that reports no key and no change: macOS repeats a Caps
    /// Lock change for every keyboard, and again before the next key. Browsers turn
    /// these into stray key events.
    CapsLockRepeated,
}

/// What the tap does with an event.
#[derive(Debug, PartialEq)]
pub enum TapAction {
    Forward,
    Consume,
    /// Forward a `TapEvent::CapsLockChanged` as the event a real Caps Lock toggle
    /// produces, so every app sees exactly what it would without Chord.
    ForwardAsCapsLockKey,
}

impl From<KeyEventAction> for TapAction {
    fn from(action: KeyEventAction) -> Self {
        match action {
            KeyEventAction::Forward => Self::Forward,
            KeyEventAction::Consume => Self::Consume,
        }
    }
}

/// Receives each decoded event with the time the keyboard driver stamped on it, in
/// nanoseconds (see `caps_space_gap::event_time_ns`).
pub type TapSink = Box<dyn FnMut(TapEvent, u64) -> TapAction + Send>;

/// State handed to the C callback. Lives on the tap thread for as long as the
/// tap exists; the callback only runs on that thread.
struct TapContext {
    tap: Option<CFRetained<CFMachPort>>,
    sink: TapSink,
    health: Arc<InputHealth>,
    /// Set when the tap is being stopped: from then on it passes everything through,
    /// so a replacement tap never handles the same event a second time.
    stop: Arc<AtomicBool>,
    disabled_notifications: u32,
    /// Flags of the last keyboard event from a device, to tell what a flagsChanged
    /// event changed.
    previous_flags: Option<u64>,
    /// Nanoseconds per Mach tick, as numerator and denominator.
    timebase: (u32, u32),
}

impl TapContext {
    fn handle(&mut self, kind: CGEventType, event: NonNull<CGEvent>) -> KeyEventAction {
        if self.stop.load(Ordering::Acquire) {
            return KeyEventAction::Forward;
        }
        if kind == CGEventType::TapDisabledByTimeout || kind == CGEventType::TapDisabledByUserInput
        {
            self.disabled_notifications += 1;
            if let Some(tap) = &self.tap {
                CGEvent::tap_enable(tap, true);
            }
            self.health.set_tap_enabled(true);
            let reason = if kind == CGEventType::TapDisabledByTimeout {
                "the callback took too long"
            } else {
                "macOS user-input safety cutoff"
            };
            log::warn!(
                "Keyboard event tap was disabled by macOS ({reason}); re-enabled it (occurrence #{})",
                self.disabled_notifications
            );
            return KeyEventAction::Forward;
        }

        // SAFETY: Core Graphics hands the callback a live event for the duration of the call.
        let event = unsafe { event.as_ref() };
        let user_data =
            CGEvent::integer_value_field(Some(event), CGEventField::EventSourceUserData);
        if SYNTHETIC_EVENT_USER_DATA.contains(&user_data) {
            return KeyEventAction::Forward;
        }

        let keycode = CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventKeycode);
        let autorepeat =
            CGEvent::integer_value_field(Some(event), CGEventField::KeyboardEventAutorepeat) != 0;
        let flags = CGEvent::flags(Some(event)).bits();
        let decoded = decode_tap_event(kind, keycode, flags, autorepeat, self.previous_flags);
        self.previous_flags = Some(flags);
        let Some(decoded) = decoded else {
            return KeyEventAction::Forward;
        };
        // SAFETY: reads the clock.
        #[allow(deprecated)]
        let now_ticks = unsafe { libc::mach_absolute_time() };
        let event_ns = event_time_ns(
            CGEvent::timestamp(Some(event)),
            now_ticks,
            self.timebase.0,
            self.timebase.1,
        );
        let action = (self.sink)(decoded, event_ns);
        // The hotkey fallback fires for every Caps event that is not swallowed here,
        // and for Space while it has that registered. Counting the ones let through
        // tells it those apart from the ones this tap never received.
        if (kind == CGEventType::KeyDown || kind == CGEventType::KeyUp)
            && action != TapAction::Consume
        {
            match keycode {
                REMAPPED_KEYCODE => self.health.note_caps_event_forwarded(),
                SPACE_KEYCODE => self.health.note_space_event_forwarded(),
                _ => {}
            }
        }
        match action {
            TapAction::Forward => KeyEventAction::Forward,
            TapAction::Consume => KeyEventAction::Consume,
            TapAction::ForwardAsCapsLockKey => {
                // Rewritten in place, so it keeps its position, time and flags.
                CGEvent::set_integer_value_field(
                    Some(event),
                    CGEventField::KeyboardEventKeycode,
                    CAPS_LOCK_KEYCODE,
                );
                KeyEventAction::Forward
            }
        }
    }
}

unsafe extern "C-unwind" fn tap_callback(
    _proxy: CGEventTapProxy,
    kind: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: `user_info` is the `TapContext` boxed by the owning thread, which
    // outlives the tap and never touches the context while the run loop runs.
    let Some(context) = (unsafe { user_info.cast::<TapContext>().as_mut() }) else {
        return event.as_ptr();
    };
    // A panic must never unwind into Core Graphics: that would abort the process.
    match catch_unwind(AssertUnwindSafe(|| context.handle(kind, event))) {
        Ok(KeyEventAction::Consume) => std::ptr::null_mut(),
        Ok(KeyEventAction::Forward) => event.as_ptr(),
        Err(_) => {
            log::error!("Keyboard event tap callback panicked; forwarding the event");
            event.as_ptr()
        }
    }
}

fn key_from_keycode(keycode: i64) -> Option<Key> {
    let keycode = u16::try_from(keycode).ok()?;
    Key::try_from(mac_keycode::Key::from_keycode(keycode)?).ok()
}

pub(super) fn decode_tap_event(
    kind: CGEventType,
    keycode: i64,
    flags: u64,
    autorepeat: bool,
    previous_flags: Option<u64>,
) -> Option<TapEvent> {
    // The physical Caps key, remapped so macOS never toggles Caps Lock itself. It
    // arrives here in order with every other key; see `caps_remap`.
    let caps = Key(KeyMappingCode::CapsLock);
    match kind {
        CGEventType::KeyDown | CGEventType::KeyUp if keycode == REMAPPED_KEYCODE => {
            Some(match (kind == CGEventType::KeyDown, autorepeat) {
                (true, true) => TapEvent::RemappedCapsRepeat,
                (true, false) => TapEvent::CapsPressed {
                    caps_lock_on: flags & NX_ALPHASHIFTMASK != 0,
                },
                (false, _) => TapEvent::Key(KeyEvent::Release(caps)),
            })
        }
        CGEventType::KeyDown => key_from_keycode(keycode)
            .map(KeyEvent::Press)
            .map(TapEvent::Key),
        CGEventType::KeyUp => key_from_keycode(keycode)
            .map(KeyEvent::Release)
            .map(TapEvent::Key),
        CGEventType::FlagsChanged if keycode == CAPS_LOCK_KEYCODE => {
            Some(TapEvent::NativeCapsToggle)
        }
        // A real modifier always changes its own bits, so an event that changes only
        // Caps Lock is a lock-state report, whatever keycode it carries.
        CGEventType::FlagsChanged
            if previous_flags.is_some_and(|previous| {
                (previous ^ flags) & HELD_STATE_MASK == NX_ALPHASHIFTMASK
            }) =>
        {
            Some(TapEvent::CapsLockChanged)
        }
        CGEventType::FlagsChanged => Some(
            decode_flags_changed(keycode, flags).map_or(TapEvent::CapsLockRepeated, TapEvent::Key),
        ),
        _ => None,
    }
}

/// Turns a modifier `flagsChanged` event into a press or release.
///
/// Caps Lock (keycode 57) is not a held modifier: the physical key arrives
/// remapped as F20, and keycode 57 only reports a toggle macOS made itself.
pub(super) fn decode_flags_changed(keycode: i64, flags: u64) -> Option<KeyEvent> {
    use mac_keycode::Key as K;
    let key = K::from_keycode(u16::try_from(keycode).ok()?)?;
    let (own_bit, sibling_bit, generic_bit) = match key {
        K::Shift => (NX_DEVICELSHIFTKEYMASK, NX_DEVICERSHIFTKEYMASK, NX_SHIFTMASK),
        K::RightShift => (NX_DEVICERSHIFTKEYMASK, NX_DEVICELSHIFTKEYMASK, NX_SHIFTMASK),
        K::Control => (NX_DEVICELCTLKEYMASK, NX_DEVICERCTLKEYMASK, NX_CONTROLMASK),
        K::RightControl => (NX_DEVICERCTLKEYMASK, NX_DEVICELCTLKEYMASK, NX_CONTROLMASK),
        K::Option => (NX_DEVICELALTKEYMASK, NX_DEVICERALTKEYMASK, NX_ALTERNATEMASK),
        K::RightOption => (NX_DEVICERALTKEYMASK, NX_DEVICELALTKEYMASK, NX_ALTERNATEMASK),
        K::Command => (NX_DEVICELCMDKEYMASK, NX_DEVICERCMDKEYMASK, NX_COMMANDMASK),
        K::RightCommand => (NX_DEVICERCMDKEYMASK, NX_DEVICELCMDKEYMASK, NX_COMMANDMASK),
        K::Function => (NX_SECONDARYFNMASK, 0, NX_SECONDARYFNMASK),
        _ => return None,
    };
    // Virtual keyboards may omit the device-dependent bits; fall back to the
    // generic modifier bit only when neither side is reported.
    let pressed = if flags & (own_bit | sibling_bit) != 0 {
        flags & own_bit != 0
    } else {
        flags & generic_bit != 0
    };
    let key = Key::try_from(key).ok()?;
    Some(if pressed {
        KeyEvent::Press(key)
    } else {
        KeyEvent::Release(key)
    })
}

/// The modifier keys held according to modifier flags, for a listener that starts
/// partway through and never saw them go down.
pub(super) fn held_modifier_keys(flags: u64) -> Vec<Key> {
    use KeyMappingCode::*;
    // Generic bit, then each side's device bit and key.
    let modifiers = [
        (
            NX_SHIFTMASK,
            NX_DEVICELSHIFTKEYMASK,
            ShiftLeft,
            NX_DEVICERSHIFTKEYMASK,
            ShiftRight,
        ),
        (
            NX_CONTROLMASK,
            NX_DEVICELCTLKEYMASK,
            ControlLeft,
            NX_DEVICERCTLKEYMASK,
            ControlRight,
        ),
        (
            NX_ALTERNATEMASK,
            NX_DEVICELALTKEYMASK,
            AltLeft,
            NX_DEVICERALTKEYMASK,
            AltRight,
        ),
        (
            NX_COMMANDMASK,
            NX_DEVICELCMDKEYMASK,
            MetaLeft,
            NX_DEVICERCMDKEYMASK,
            MetaRight,
        ),
    ];
    let mut held = Vec::new();
    for (generic, left_bit, left, right_bit, right) in modifiers {
        if flags & left_bit != 0 {
            held.push(Key(left));
        }
        if flags & right_bit != 0 {
            held.push(Key(right));
        }
        // Virtual keyboards may omit the device-dependent bits.
        if flags & (left_bit | right_bit) == 0 && flags & generic != 0 {
            held.push(Key(left));
        }
    }
    if flags & NX_SECONDARYFNMASK != 0 {
        held.push(Key(Fn));
    }
    held
}

/// A running event tap thread. Dropping the handle does not stop the tap;
/// call [`TapHandle::stop`].
pub struct TapHandle {
    thread: JoinHandle<()>,
    stop: Arc<AtomicBool>,
    run_loop: TapRunLoop,
}

/// The tap thread's run loop, kept to wake that thread when the tap is stopped.
struct TapRunLoop(CFRetained<CFRunLoop>);

// SAFETY: only `CFRunLoopStop` is called through it, which Core Foundation documents
// as safe to call from any thread.
unsafe impl Send for TapRunLoop {}

impl TapHandle {
    /// Creates the tap on a dedicated thread and returns once it is enabled.
    pub fn spawn(health: Arc<InputHealth>, sink: TapSink) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = sync_channel::<Result<TapRunLoop>>(1);
        let thread = std::thread::Builder::new()
            .name("chord-key-tap".into())
            .spawn({
                let stop = stop.clone();
                move || run_tap(health, sink, stop, ready_tx)
            })
            .context("spawn keyboard event tap thread")?;
        match ready_rx.recv_timeout(START_TIMEOUT) {
            Ok(Ok(run_loop)) => Ok(Self {
                thread,
                stop,
                run_loop,
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                Err(anyhow!(
                    "keyboard event tap did not start within {START_TIMEOUT:?}"
                ))
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }

    /// Stops the tap and waits for its thread. The tap handles no event after the
    /// flag is set, and the wake-up makes the thread notice it at once.
    pub fn stop(self) {
        self.stop.store(true, Ordering::Release);
        self.run_loop.0.stop();
        let _ = self.thread.join();
    }
}

fn run_tap(
    health: Arc<InputHealth>,
    sink: TapSink,
    stop: Arc<AtomicBool>,
    ready_tx: std::sync::mpsc::SyncSender<Result<TapRunLoop>>,
) {
    #[allow(deprecated)]
    let mut timebase = libc::mach_timebase_info { numer: 1, denom: 1 };
    // SAFETY: the output points to live storage.
    #[allow(deprecated)]
    unsafe {
        libc::mach_timebase_info(&mut timebase)
    };
    let mut context = Box::new(TapContext {
        tap: None,
        sink,
        health: health.clone(),
        stop: stop.clone(),
        disabled_notifications: 0,
        previous_flags: None,
        timebase: (timebase.numer.max(1), timebase.denom.max(1)),
    });
    let events_of_interest = (1u64 << CGEventType::KeyDown.0)
        | (1u64 << CGEventType::KeyUp.0)
        | (1u64 << CGEventType::FlagsChanged.0);
    // SAFETY: the callback is a valid `CGEventTapCallBack` and the context
    // pointer stays valid until the tap is invalidated below.
    let tap = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::HIDEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            events_of_interest,
            Some(tap_callback),
            (&mut *context as *mut TapContext).cast(),
        )
    };
    let Some(tap) = tap else {
        let _ = ready_tx.send(Err(anyhow!(
            "CGEventTapCreate failed: Accessibility is not granted to this executable"
        )));
        return;
    };
    context.tap = Some(tap.clone());
    let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
        tap.invalidate();
        let _ = ready_tx.send(Err(anyhow!("CFMachPortCreateRunLoopSource failed")));
        return;
    };
    let Some(run_loop) = CFRunLoop::current() else {
        tap.invalidate();
        let _ = ready_tx.send(Err(anyhow!("no run loop on the event tap thread")));
        return;
    };
    // SAFETY: reading a Core Foundation constant.
    let (common_modes, default_mode) = unsafe { (kCFRunLoopCommonModes, kCFRunLoopDefaultMode) };
    run_loop.add_source(Some(&source), common_modes);
    CGEvent::tap_enable(&tap, true);
    health.set_tap_running(true);
    health.set_tap_enabled(true);
    log::info!("Keyboard event tap started");
    let _ = ready_tx.send(Ok(TapRunLoop(run_loop.clone())));

    let started = Instant::now();
    let mut reenabled_by_watchdog = 0u32;
    let exit_reason = loop {
        if stop.load(Ordering::Acquire) {
            break None;
        }
        let result =
            CFRunLoop::run_in_mode(default_mode, HEALTH_CHECK_INTERVAL.as_secs_f64(), false);
        if result == CFRunLoopRunResult::Finished {
            break Some("the run loop has no sources left".to_string());
        }
        // Belt and braces: the disable notification is not always delivered
        // (notably around sleep/wake), so verify the tap state directly.
        let enabled = CGEvent::tap_is_enabled(&tap);
        if !enabled {
            CGEvent::tap_enable(&tap, true);
            reenabled_by_watchdog += 1;
            log::warn!(
                "Keyboard event tap was found disabled without a notification; re-enabled it (occurrence #{reenabled_by_watchdog})"
            );
        }
        health.set_tap_enabled(CGEvent::tap_is_enabled(&tap));
    };

    CGEvent::tap_enable(&tap, false);
    run_loop.remove_source(Some(&source), common_modes);
    tap.invalidate();
    context.tap = None;
    health.set_tap_running(false);
    match exit_reason {
        None => log::info!(
            "Keyboard event tap stopped on request after {:?}",
            started.elapsed()
        ),
        Some(reason) => log::error!(
            "Keyboard event tap stopped after {:?}: {reason}",
            started.elapsed()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycode::KeyMappingCode;

    const SHIFT: i64 = 0x38;
    const RIGHT_SHIFT: i64 = 0x3C;
    const CONTROL: i64 = 0x3B;
    const RIGHT_COMMAND: i64 = 0x36;
    const FUNCTION: i64 = 0x3F;
    const CAPS_LOCK: i64 = 0x39;

    fn key(code: KeyMappingCode) -> Key {
        Key(code)
    }

    #[test]
    fn decodes_left_and_right_modifiers_from_device_bits() {
        assert_eq!(
            decode_flags_changed(SHIFT, NX_SHIFTMASK | NX_DEVICELSHIFTKEYMASK),
            Some(KeyEvent::Press(key(KeyMappingCode::ShiftLeft)))
        );
        // Left shift released while right shift is still held.
        assert_eq!(
            decode_flags_changed(SHIFT, NX_SHIFTMASK | NX_DEVICERSHIFTKEYMASK),
            Some(KeyEvent::Release(key(KeyMappingCode::ShiftLeft)))
        );
        assert_eq!(
            decode_flags_changed(RIGHT_SHIFT, NX_SHIFTMASK | NX_DEVICERSHIFTKEYMASK),
            Some(KeyEvent::Press(key(KeyMappingCode::ShiftRight)))
        );
        assert_eq!(
            decode_flags_changed(RIGHT_COMMAND, 0),
            Some(KeyEvent::Release(key(KeyMappingCode::MetaRight)))
        );
        assert_eq!(
            decode_flags_changed(FUNCTION, NX_SECONDARYFNMASK),
            Some(KeyEvent::Press(key(KeyMappingCode::Fn)))
        );
        assert_eq!(
            decode_flags_changed(FUNCTION, 0),
            Some(KeyEvent::Release(key(KeyMappingCode::Fn)))
        );
    }

    #[test]
    fn falls_back_to_generic_bits_for_virtual_keyboards() {
        assert_eq!(
            decode_flags_changed(CONTROL, NX_CONTROLMASK),
            Some(KeyEvent::Press(key(KeyMappingCode::ControlLeft)))
        );
        assert_eq!(
            decode_flags_changed(CONTROL, 0),
            Some(KeyEvent::Release(key(KeyMappingCode::ControlLeft)))
        );
    }

    #[test]
    fn caps_lock_and_unknown_keycodes_are_not_modifier_presses() {
        assert_eq!(decode_flags_changed(CAPS_LOCK, 0x1_0000), None);
        assert_eq!(decode_flags_changed(0x7FFF, NX_SHIFTMASK), None);
        assert_eq!(decode_flags_changed(-1, NX_SHIFTMASK), None);
    }

    #[test]
    fn maps_virtual_keycodes_to_chord_keys() {
        assert_eq!(key_from_keycode(0x31), Some(key(KeyMappingCode::Space)));
        assert_eq!(key_from_keycode(0x39), Some(key(KeyMappingCode::CapsLock)));
        assert_eq!(key_from_keycode(0x00), Some(key(KeyMappingCode::KeyA)));
        assert_eq!(key_from_keycode(0x7FFF), None);
    }

    const OFF: u64 = 0x100;
    const ON: u64 = 0x100 | NX_ALPHASHIFTMASK;

    #[test]
    fn a_lock_change_is_reported_once_as_the_caps_lock_key() {
        // What one IOHIDSetModifierLockState call produced on a Mac with two
        // keyboards: keycode 255 from each, then the next key's code before that key.
        let reports = [(0xFF, ON), (0xFF, ON), (0x31, ON)];
        let mut previous = OFF;
        let decoded: Vec<_> = reports
            .into_iter()
            .map(|(keycode, flags)| {
                let event = decode_tap_event(
                    CGEventType::FlagsChanged,
                    keycode,
                    flags,
                    false,
                    Some(previous),
                );
                previous = flags;
                event
            })
            .collect();
        assert_eq!(
            decoded,
            [
                Some(TapEvent::CapsLockChanged),
                Some(TapEvent::CapsLockRepeated),
                Some(TapEvent::CapsLockRepeated),
            ]
        );
        // The key itself follows as usual, and turning Caps Lock off works the same.
        assert_eq!(
            decode_tap_event(CGEventType::KeyUp, 0x31, ON, false, Some(ON)),
            Some(TapEvent::Key(KeyEvent::Release(key(KeyMappingCode::Space))))
        );
        assert_eq!(
            decode_tap_event(CGEventType::FlagsChanged, 0x31, OFF, false, Some(ON)),
            Some(TapEvent::CapsLockChanged)
        );
    }

    #[test]
    fn a_lock_report_is_recognized_by_its_flags_not_its_keycode() {
        // An idle keyboard whose last key was Shift reports with Shift's code.
        assert_eq!(
            decode_tap_event(CGEventType::FlagsChanged, SHIFT, ON, false, Some(OFF)),
            Some(TapEvent::CapsLockChanged)
        );
        // Bits that ride along on ordinary keys, like an arrow key's, don't count.
        let arrow = OFF | 0x0020_0000 | NX_SECONDARYFNMASK;
        assert_eq!(
            decode_tap_event(CGEventType::FlagsChanged, 0xFF, ON, false, Some(arrow)),
            Some(TapEvent::CapsLockChanged)
        );
        // Without a previous event there is nothing to compare: no report.
        assert_eq!(
            decode_tap_event(CGEventType::FlagsChanged, 0xFF, ON, false, None),
            Some(TapEvent::CapsLockRepeated)
        );
    }

    #[test]
    fn real_modifier_changes_keep_their_existing_behavior() {
        let shift = NX_SHIFTMASK | NX_DEVICELSHIFTKEYMASK;
        for caps_lock in [OFF, ON] {
            assert_eq!(
                decode_tap_event(
                    CGEventType::FlagsChanged,
                    SHIFT,
                    caps_lock | shift,
                    false,
                    Some(caps_lock)
                ),
                Some(TapEvent::Key(KeyEvent::Press(key(
                    KeyMappingCode::ShiftLeft
                ))))
            );
            assert_eq!(
                decode_tap_event(
                    CGEventType::FlagsChanged,
                    SHIFT,
                    caps_lock,
                    false,
                    Some(caps_lock | shift)
                ),
                Some(TapEvent::Key(KeyEvent::Release(key(
                    KeyMappingCode::ShiftLeft
                ))))
            );
        }
        // A modifier that changes together with Caps Lock is still the modifier.
        assert_eq!(
            decode_tap_event(
                CGEventType::FlagsChanged,
                SHIFT,
                ON | shift,
                false,
                Some(OFF)
            ),
            Some(TapEvent::Key(KeyEvent::Press(key(
                KeyMappingCode::ShiftLeft
            ))))
        );
    }

    #[test]
    fn the_remapped_caps_key_is_caps_and_never_repeats() {
        let caps = key(KeyMappingCode::CapsLock);
        assert_eq!(
            decode_tap_event(
                CGEventType::KeyDown,
                REMAPPED_KEYCODE,
                OFF,
                false,
                Some(OFF)
            ),
            Some(TapEvent::CapsPressed {
                caps_lock_on: false
            })
        );
        // The press carries the lock state from before it, which decides whether
        // Space can follow it into Chord.
        assert_eq!(
            decode_tap_event(CGEventType::KeyDown, REMAPPED_KEYCODE, ON, false, Some(ON)),
            Some(TapEvent::CapsPressed { caps_lock_on: true })
        );
        assert_eq!(
            decode_tap_event(CGEventType::KeyDown, REMAPPED_KEYCODE, OFF, true, Some(OFF)),
            Some(TapEvent::RemappedCapsRepeat)
        );
        assert_eq!(
            decode_tap_event(CGEventType::KeyUp, REMAPPED_KEYCODE, OFF, false, Some(OFF)),
            Some(TapEvent::Key(KeyEvent::Release(caps)))
        );
        // Other keys still repeat as before.
        assert_eq!(
            decode_tap_event(CGEventType::KeyDown, 0x00, OFF, true, Some(OFF)),
            Some(TapEvent::Key(KeyEvent::Press(key(KeyMappingCode::KeyA))))
        );
    }

    #[test]
    fn held_modifiers_are_read_from_the_flags() {
        assert!(held_modifier_keys(OFF).is_empty());
        assert!(
            held_modifier_keys(ON).is_empty(),
            "Caps Lock is not a held key"
        );
        assert_eq!(
            held_modifier_keys(OFF | NX_SHIFTMASK | NX_DEVICELSHIFTKEYMASK),
            [key(KeyMappingCode::ShiftLeft)]
        );
        assert_eq!(
            held_modifier_keys(
                NX_CONTROLMASK | NX_DEVICERCTLKEYMASK | NX_COMMANDMASK | NX_DEVICELCMDKEYMASK
            ),
            [
                key(KeyMappingCode::ControlRight),
                key(KeyMappingCode::MetaLeft)
            ]
        );
        assert_eq!(
            held_modifier_keys(NX_SHIFTMASK | NX_DEVICELSHIFTKEYMASK | NX_DEVICERSHIFTKEYMASK),
            [
                key(KeyMappingCode::ShiftLeft),
                key(KeyMappingCode::ShiftRight)
            ]
        );
        // Without device bits the left key stands for the modifier.
        assert_eq!(
            held_modifier_keys(NX_ALTERNATEMASK),
            [key(KeyMappingCode::AltLeft)]
        );
        assert_eq!(
            held_modifier_keys(NX_SECONDARYFNMASK),
            [key(KeyMappingCode::Fn)]
        );
    }

    #[test]
    fn a_native_caps_toggle_means_the_mapping_is_gone() {
        for (flags, previous) in [(ON, OFF), (OFF, ON)] {
            assert_eq!(
                decode_tap_event(
                    CGEventType::FlagsChanged,
                    CAPS_LOCK,
                    flags,
                    false,
                    Some(previous)
                ),
                Some(TapEvent::NativeCapsToggle)
            );
        }
    }
}
