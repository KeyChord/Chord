//! The window Chord reads a chord through while its event tap is blind.
//!
//! Under Secure Event Input macOS hides key presses from every event tap, and
//! delivers hotkeys only for keys that type nothing, so the letters of a chord can
//! reach Chord one way: as ordinary key events to a window that has the keyboard.
//! This is that window. It is invisible and takes the key focus without activating
//! Chord, so the app in front stays in front and keeps its own key window; when the
//! chord ends, the keyboard is that app's again. The Hint Panel is a different window
//! and still never takes the keyboard.
//!
//! The keys Chord simulates would land here too while this window has the keyboard.
//! They are passed on to the app in front, which is where they were going.
//!
//! Everything here runs on the main thread.

use super::tap::{SYNTHETIC_EVENT_USER_DATA, decode_tap_event};
use crate::app::AppHandleExt;
use anyhow::{Context, Result, ensure};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSEventType, NSFloatingWindowLevel, NSPanel,
    NSResponder, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior,
    NSWindowDidResignKeyNotification, NSWindowStyleMask, NSWorkspace,
};
use objc2_core_graphics::{CGEvent, CGEventField, CGEventType};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize};
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

define_class!(
    // SAFETY: `NSPanel` has no subclassing requirements beyond overriding its
    // methods with matching signatures, and this class adds no state and no `Drop`.
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ChordKeyCatcher"]
    struct KeyCatcher;

    impl KeyCatcher {
        // A borderless panel refuses the keyboard unless it says otherwise.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// The window with the monitors that feed and watch it, which live as long as it does.
struct Catcher {
    panel: Retained<KeyCatcher>,
    _key_monitor: Option<Retained<AnyObject>>,
    _focus_observer: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

thread_local! {
    static CATCHER: RefCell<Option<Catcher>> = const { RefCell::new(None) };
    /// Flags of the previous event the window received, to tell what a
    /// flagsChanged event changed (see `decode_tap_event`).
    static PREVIOUS_FLAGS: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Whether the window has been given the keyboard and not yet taken out again.
static SHOWN: AtomicBool = AtomicBool::new(false);

impl Catcher {
    fn new(handle: &AppHandle, mtm: MainThreadMarker) -> Result<Self> {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
        let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
        // SAFETY: the designated initializer of `NSWindow`, with arguments of its
        // declared types, on the main thread.
        let panel: Retained<KeyCatcher> = unsafe {
            msg_send![
                KeyCatcher::alloc(mtm),
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // SAFETY: the window is owned by `Retained` handles, never by its own close.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setIgnoresMouseEvents(true);
        panel.setHidesOnDeactivate(false);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setWorksWhenModal(true);
        panel.setLevel(NSFloatingWindowLevel);
        panel.setAnimationBehavior(NSWindowAnimationBehavior::None);
        // Like the Hint Panel: on whichever Space, including beside a full-screen app.
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );

        let for_keys = handle.clone();
        let on_key = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit passes a live event for the duration of the call.
            if take_event(&for_keys, unsafe { event.as_ref() }) {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        let mask = NSEventMask::KeyDown | NSEventMask::KeyUp | NSEventMask::FlagsChanged;
        // SAFETY: the block returns the event it was given or null.
        let key_monitor =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &on_key) };
        ensure!(
            key_monitor.is_some(),
            "AppKit refused the key event monitor"
        );

        let for_focus = handle.clone();
        let on_resign = RcBlock::new(move |_: NonNull<NSNotification>| {
            // Hiding the window on purpose clears the flag first.
            if SHOWN.load(Ordering::SeqCst) {
                for_focus.app_state().keyboard().on_key_catcher_lost_focus();
            }
        });
        let window: &AnyObject = &panel;
        // SAFETY: the notification is posted on the main thread, where the block may
        // run; the name is a static and the object outlives the observer.
        let focus_observer = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(NSWindowDidResignKeyNotification),
                Some(window),
                None,
                &on_resign,
            )
        };
        Ok(Self {
            panel,
            _key_monitor: key_monitor,
            _focus_observer: focus_observer,
        })
    }
}

/// Gives Chord's window the keyboard, for the chord that is starting.
pub(super) fn show(handle: &AppHandle) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the key catcher is used on the main thread")?;
    let panel = CATCHER.with(|catcher| -> Result<Retained<KeyCatcher>> {
        if let Some(catcher) = catcher.borrow().as_ref() {
            return Ok(catcher.panel.clone());
        }
        let created = Catcher::new(handle, mtm)?;
        let panel = created.panel.clone();
        *catcher.borrow_mut() = Some(created);
        Ok(panel)
    })?;
    PREVIOUS_FLAGS.set(None);
    SHOWN.store(true, Ordering::SeqCst);
    panel.makeKeyAndOrderFront(None);
    ensure!(
        panel.isKeyWindow(),
        "macOS did not give Chord's window the keyboard"
    );
    Ok(())
}

/// Gives the keyboard back to the app in front. Does nothing off the main thread.
pub(super) fn hide() {
    SHOWN.store(false, Ordering::SeqCst);
    if MainThreadMarker::new().is_none() {
        return;
    }
    let panel = CATCHER.with(|catcher| catcher.borrow().as_ref().map(|c| c.panel.clone()));
    if let Some(panel) = panel {
        panel.orderOut(None);
    }
}

/// Handles a key event AppKit is about to deliver in this process. Returns whether it
/// was the catcher's, in which case nothing else gets it.
fn take_event(handle: &AppHandle, event: &NSEvent) -> bool {
    if !SHOWN.load(Ordering::SeqCst) {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let ours = CATCHER.with(|catcher| {
        let catcher = catcher.borrow();
        let (Some(catcher), Some(window)) = (catcher.as_ref(), event.window(mtm)) else {
            return false;
        };
        let panel: &NSWindow = &catcher.panel;
        std::ptr::eq(panel, &*window)
    });
    if !ours {
        return false;
    }
    let keyboard = handle.app_state().keyboard();
    if let Some(event) = event.CGEvent() {
        let simulated = SYNTHETIC_EVENT_USER_DATA.contains(&CGEvent::integer_value_field(
            Some(&event),
            CGEventField::EventSourceUserData,
        ));
        // The chord can end a moment before this window is taken out; what is typed
        // in between is the app's, like the keys Chord simulates.
        if simulated || !keyboard.is_catching_keys() {
            pass_on_to_front_app(&event);
            return true;
        }
    }
    let kind = event.r#type();
    let kind = if kind == NSEventType::KeyDown {
        CGEventType::KeyDown
    } else if kind == NSEventType::KeyUp {
        CGEventType::KeyUp
    } else if kind == NSEventType::FlagsChanged {
        CGEventType::FlagsChanged
    } else {
        return true;
    };
    // The same fields, with the same values, as on the event a tap receives.
    let keycode = i64::from(event.keyCode());
    let flags = event.modifierFlags().0 as u64;
    // Asking a flagsChanged event whether it repeats raises an exception.
    let autorepeat = kind == CGEventType::KeyDown && event.isARepeat();
    let previous = PREVIOUS_FLAGS.replace(Some(flags));
    if let Some(decoded) = decode_tap_event(kind, keycode, flags, autorepeat, previous) {
        keyboard.handle_caught_event(decoded);
    }
    true
}

/// A key that is not for Chord arrived here because this window has the keyboard. The
/// app in front still has its own key window, and gets the key there.
fn pass_on_to_front_app(event: &CGEvent) {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return;
    };
    let pid = app.processIdentifier();
    // With Chord itself in front, sending it back would only return it here.
    if pid == std::process::id() as libc::pid_t {
        return;
    }
    CGEvent::post_to_pid(pid, Some(event));
}
