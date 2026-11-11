//! Keeps the Caps key and Caps+Space working where Chord's event tap is blind.
//!
//! Secure Event Input (password fields, Secure Keyboard Entry, apps that leave it on)
//! withholds key presses from every event tap, but WindowServer still delivers Carbon
//! hotkeys for keys that type nothing on their own, which includes the remapped Caps
//! key and Space. While the tap sees the Caps key it swallows it, so the Caps hotkey
//! only fires when the tap cannot: no Secure Input detection is needed.
//!
//! That also makes the hotkey a witness for the tap. A tap can stop receiving events
//! while macOS still reports it as enabled; a Caps event that arrives here without
//! having passed through the tap, with Secure Input off, shows exactly that, and the
//! tap is replaced (see `AppKeyboard::handle_unseen_caps_event`).
//!
//! Space is a hotkey only while such a blind Caps press is held and can start Chord,
//! and until a Space press Chord took is released. A Space press that turns out not to
//! be Chord's is handed back to the focused app and the registration dropped, so a
//! stale registration costs one keystroke a detour and never the keystroke. The rest
//! of a chord cannot arrive as hotkeys (macOS does not deliver them for letters under
//! Secure Input); `key_catcher` reads it.

use super::tap::SPACE_KEYCODE;
use crate::app::AppHandleExt;
use crate::models::{Key, KeyEvent};
use anyhow::{Result, ensure};
use keycode::KeyMappingCode;
use objc2_core_graphics::{CGEvent, CGEventSource, CGEventSourceStateID, CGEventTapLocation};
use objc2_foundation::MainThreadMarker;
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::AppHandle;

type OSStatus = i32;
type EventTargetRef = *mut c_void;
type EventHandlerCallRef = *mut c_void;
type EventRef = *mut c_void;
type EventHotKeyRef = *mut c_void;

#[repr(C)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EventHotKeyID {
    signature: u32,
    id: u32,
}

type EventHandler = extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> EventTargetRef;
    fn InstallEventHandler(
        target: EventTargetRef,
        handler: EventHandler,
        num_types: u32,
        list: *const EventTypeSpec,
        user_data: *mut c_void,
        out_ref: *mut *mut c_void,
    ) -> OSStatus;
    fn RegisterEventHotKey(
        key_code: u32,
        modifiers: u32,
        id: EventHotKeyID,
        target: EventTargetRef,
        options: u32,
        out_ref: *mut EventHotKeyRef,
    ) -> OSStatus;
    fn UnregisterEventHotKey(hot_key: EventHotKeyRef) -> OSStatus;
    fn GetEventKind(event: EventRef) -> u32;
    fn GetEventParameter(
        event: EventRef,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        buffer_size: usize,
        actual_size: *mut usize,
        data: *mut c_void,
    ) -> OSStatus;
}

const K_EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
const K_EVENT_HOT_KEY_RELEASED: u32 = 6;
const K_EVENT_PARAM_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
const TYPE_EVENT_HOT_KEY_ID: u32 = u32::from_be_bytes(*b"hkid");
const EVENT_NOT_HANDLED_ERR: OSStatus = -9874;
const SIGNATURE: u32 = u32::from_be_bytes(*b"CHRD");
/// Carbon modifier masks each hotkey is registered for: none and Shift (`shiftKey`),
/// since a hotkey only matches its exact modifiers.
const MODIFIER_VARIANTS: [u32; 2] = [0, 1 << 9];
/// Hotkey IDs: a base per key, plus the index of the modifier variant.
const CAPS_ID: u32 = 1;
const SPACE_ID: u32 = 11;

static HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Registered hotkeys. Only touched on the main thread; stored as addresses.
struct Registered {
    caps: Vec<usize>,
    space: Vec<usize>,
}

static REGISTERED: parking_lot::Mutex<Registered> = parking_lot::Mutex::new(Registered {
    caps: Vec::new(),
    space: Vec::new(),
});

/// `InputHealth::caps_events_forwarded` as of the previous Caps hotkey event, or of the
/// registration: events let through while the hotkey was not registered don't count.
static TAP_CAPS_FORWARDS_SEEN: AtomicU64 = AtomicU64::new(0);
/// The same for Space.
static TAP_SPACE_FORWARDS_SEEN: AtomicU64 = AtomicU64::new(0);

/// The Caps press in progress reached Chord through the hotkey: the tap is blind and
/// would miss the Space that follows.
static BLIND_CAPS_HELD: AtomicBool = AtomicBool::new(false);
/// Chord took the Space press in progress, so its release is Chord's as well.
static SPACE_TAKEN: AtomicBool = AtomicBool::new(false);
/// Whether Space is registered, for the listener threads: they only bother the main
/// thread when there is something to undo.
static SPACE_REGISTERED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_hotkey(_: EventHandlerCallRef, event: EventRef, _: *mut c_void) -> OSStatus {
    let mut id = EventHotKeyID {
        signature: 0,
        id: 0,
    };
    // SAFETY: Carbon passes a live event for the duration of the callback, and the
    // output is exactly one hotkey ID.
    let status = unsafe {
        GetEventParameter(
            event,
            K_EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOT_KEY_ID,
            std::ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>(),
            std::ptr::null_mut(),
            (&mut id as *mut EventHotKeyID).cast(),
        )
    };
    // Other hotkeys in this process belong to other handlers.
    let (true, Some(handle)) = (status == 0 && id.signature == SIGNATURE, HANDLE.get()) else {
        return EVENT_NOT_HANDLED_ERR;
    };
    // SAFETY: as above.
    let pressed = match unsafe { GetEventKind(event) } {
        K_EVENT_HOT_KEY_PRESSED => true,
        K_EVENT_HOT_KEY_RELEASED => false,
        _ => return EVENT_NOT_HANDLED_ERR,
    };
    let variants = MODIFIER_VARIANTS.len() as u32;
    if (CAPS_ID..CAPS_ID + variants).contains(&id.id) {
        handle_caps(handle, pressed);
    } else if (SPACE_ID..SPACE_ID + variants).contains(&id.id) {
        handle_space(handle, pressed);
    } else {
        return EVENT_NOT_HANDLED_ERR;
    }
    0
}

fn handle_caps(handle: &AppHandle, pressed: bool) {
    let keyboard = handle.app_state().keyboard();
    // The tap runs before hotkeys are matched. It swallows the key while Chord owns
    // input, so this event either was let through by it (during an ownership handoff),
    // which moved its count, or never reached it.
    let forwarded = keyboard.health().caps_events_forwarded();
    let tap_received = TAP_CAPS_FORWARDS_SEEN.swap(forwarded, Ordering::AcqRel) != forwarded;
    log::debug!(
        "Caps through the hotkey fallback: pressed={pressed}, received by the event tap: {tap_received}"
    );
    BLIND_CAPS_HELD.store(pressed && !tap_received, Ordering::SeqCst);
    let key = Key(KeyMappingCode::CapsLock);
    let event = if pressed {
        KeyEvent::Press(key)
    } else {
        KeyEvent::Release(key)
    };
    keyboard.handle_unseen_caps_event(event, tap_received);
    sync_space(handle);
}

fn handle_space(handle: &AppHandle, pressed: bool) {
    let keyboard = handle.app_state().keyboard();
    if !pressed {
        if SPACE_TAKEN.swap(false, Ordering::SeqCst) {
            keyboard.handle_unseen_space_release();
        }
        sync_space(handle);
        return;
    }
    // As for Caps: a press the tap saw and let through was not Chord's when it passed.
    let forwarded = keyboard.health().space_events_forwarded();
    let tap_received = TAP_SPACE_FORWARDS_SEEN.swap(forwarded, Ordering::AcqRel) != forwarded;
    let taken = !tap_received && keyboard.handle_unseen_space_press();
    log::debug!(
        "Space through the hotkey fallback: taken={taken}, received by the event tap: {tap_received}"
    );
    SPACE_TAKEN.store(taken, Ordering::SeqCst);
    if taken {
        return;
    }
    // The registration was stale. Drop it first, so that the release and whatever is
    // typed next go straight to the focused app, then give this press back.
    BLIND_CAPS_HELD.store(false, Ordering::SeqCst);
    sync_space(handle);
    give_back_space_press();
}

/// Posts the Space press the hotkey swallowed, past the stage where hotkeys are
/// matched: the focused app receives it and no hotkey sees it again.
fn give_back_space_press() {
    let state = CGEventSourceStateID::HIDSystemState;
    let source = CGEventSource::new(state);
    let Some(event) = CGEvent::new_keyboard_event(source.as_deref(), SPACE_KEYCODE as u16, true)
    else {
        log::error!("Could not give a Space press back to the focused app");
        return;
    };
    CGEvent::set_flags(Some(&event), CGEventSource::flags_state(state));
    CGEvent::post(CGEventTapLocation::AnnotatedSessionEventTap, Some(&event));
}

/// Registers Space while Chord would take the next Space press and the tap cannot see
/// it, and unregisters it otherwise. Must run on the main thread.
fn sync_space(handle: &AppHandle) {
    let keyboard = handle.app_state().keyboard();
    let mut registered = REGISTERED.lock();
    // Only the input owner has the Caps hotkey, and only it listens for Space.
    let wanted = !registered.caps.is_empty()
        && (SPACE_TAKEN.load(Ordering::SeqCst)
            || (BLIND_CAPS_HELD.load(Ordering::SeqCst) && keyboard.state().is_caps_armed()));
    if wanted == !registered.space.is_empty() {
        return;
    }
    if !wanted {
        unregister(&mut registered.space);
        SPACE_REGISTERED.store(false, Ordering::SeqCst);
        return;
    }
    TAP_SPACE_FORWARDS_SEEN.store(
        keyboard.health().space_events_forwarded(),
        Ordering::Release,
    );
    registered.space = register(SPACE_KEYCODE as u32, SPACE_ID, "Space");
    SPACE_REGISTERED.store(!registered.space.is_empty(), Ordering::SeqCst);
}

/// Whether Chord took the Space press in progress through the hotkey. Its release then
/// belongs to the hotkey as well, whoever else sees it first.
pub(super) fn space_press_taken() -> bool {
    SPACE_TAKEN.load(Ordering::SeqCst)
}

/// A listener received the release of the Caps key: no Space is awaited after that
/// press. Safe from any thread; cheap unless Space is registered.
pub(super) fn caps_released(handle: &AppHandle) {
    if !SPACE_REGISTERED.load(Ordering::SeqCst) {
        return;
    }
    BLIND_CAPS_HELD.store(false, Ordering::SeqCst);
    on_main_thread(handle, sync_space);
}

/// Chord mode ended, however it did: a Space press taken for it is no longer Chord's.
/// Safe from any thread; cheap unless Space is registered.
pub(super) fn chord_ended(handle: &AppHandle) {
    if !SPACE_REGISTERED.load(Ordering::SeqCst) {
        return;
    }
    SPACE_TAKEN.store(false, Ordering::SeqCst);
    on_main_thread(handle, sync_space);
}

fn on_main_thread(handle: &AppHandle, operation: fn(&AppHandle)) {
    if MainThreadMarker::new().is_some() {
        operation(handle);
        return;
    }
    let for_main = handle.clone();
    if let Err(error) = handle.run_on_main_thread(move || operation(&for_main)) {
        log::warn!("Failed to schedule a hotkey fallback update: {error:#}");
    }
}

fn register(key_code: u32, first_id: u32, name: &str) -> Vec<usize> {
    let mut hotkeys = Vec::new();
    for (index, modifiers) in MODIFIER_VARIANTS.into_iter().enumerate() {
        let mut hotkey: EventHotKeyRef = std::ptr::null_mut();
        // SAFETY: the output points to live storage; the target is the app's.
        let status = unsafe {
            RegisterEventHotKey(
                key_code,
                modifiers,
                EventHotKeyID {
                    signature: SIGNATURE,
                    id: first_id + index as u32,
                },
                GetApplicationEventTarget(),
                0,
                &mut hotkey,
            )
        };
        if status != 0 {
            log::warn!(
                "Could not register the {name} fallback hotkey (modifiers {modifiers:#x}): {status}"
            );
            continue;
        }
        hotkeys.push(hotkey as usize);
    }
    hotkeys
}

fn unregister(hotkeys: &mut Vec<usize>) {
    for hotkey in hotkeys.drain(..) {
        // SAFETY: registered by this module on the main thread.
        unsafe { UnregisterEventHotKey(hotkey as EventHotKeyRef) };
    }
}

/// Registers or unregisters the fallback. Must run on the main thread.
pub(super) fn set_registered(handle: &AppHandle, registered: bool) -> Result<()> {
    let mut hotkeys = REGISTERED.lock();
    if registered == !hotkeys.caps.is_empty() {
        return Ok(());
    }
    if !registered {
        unregister(&mut hotkeys.caps);
        unregister(&mut hotkeys.space);
        BLIND_CAPS_HELD.store(false, Ordering::SeqCst);
        SPACE_TAKEN.store(false, Ordering::SeqCst);
        SPACE_REGISTERED.store(false, Ordering::SeqCst);
        return Ok(());
    }
    TAP_CAPS_FORWARDS_SEEN.store(
        handle
            .app_state()
            .keyboard()
            .health()
            .caps_events_forwarded(),
        Ordering::Release,
    );
    if HANDLE.set(handle.clone()).is_ok() {
        let types = [
            EventTypeSpec {
                event_class: K_EVENT_CLASS_KEYBOARD,
                event_kind: K_EVENT_HOT_KEY_PRESSED,
            },
            EventTypeSpec {
                event_class: K_EVENT_CLASS_KEYBOARD,
                event_kind: K_EVENT_HOT_KEY_RELEASED,
            },
        ];
        // SAFETY: a static handler and a stack array Carbon copies.
        let status = unsafe {
            InstallEventHandler(
                GetApplicationEventTarget(),
                handle_hotkey,
                types.len() as u32,
                types.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        ensure!(status == 0, "InstallEventHandler failed: {status}");
    }
    hotkeys.caps = register(super::caps_remap::REMAPPED_KEYCODE as u32, CAPS_ID, "Caps");
    Ok(())
}
