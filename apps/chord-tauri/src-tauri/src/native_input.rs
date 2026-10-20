//! Window-targeted mouse input shared by Rust callers and the built-in `chord` module.
//! Coordinates are top-left window-local macOS points. Cursor movement is optional; no activation.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl WindowBounds {
    fn is_valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite())
            && self.width >= 1.0
            && self.height >= 1.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeWindow {
    pub window_id: u32,
    pub process_id: i32,
    pub bounds: WindowBounds,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickOptions {
    pub window_id: u32,
    pub x: f64,
    pub y: f64,
    /// If supplied, reject a window that moved or resized since the caller resolved coordinates.
    pub expected_bounds: Option<WindowBounds>,
    /// Move the cursor to the resolved screen point while keeping clicks window-targeted.
    #[serde(default)]
    pub move_cursor: bool,
    #[serde(default)]
    pub button: MouseButton,
    #[serde(default = "default_click_count")]
    pub click_count: u8,
    #[serde(default = "default_press_duration")]
    pub press_duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseAction {
    Click,
    Down,
    Move,
    Up,
}

/// A missing window ID selects ordinary global input, including normal focus changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MouseOptions {
    pub action: MouseAction,
    pub window_id: Option<u32>,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    pub expected_bounds: Option<WindowBounds>,
    #[serde(default)]
    pub button: MouseButton,
    #[serde(default = "default_click_count")]
    pub click_count: u8,
    #[serde(default = "default_press_duration")]
    pub press_duration_ms: u64,
}

impl MouseOptions {
    fn validate(&self) -> Result<()> {
        if self.action == MouseAction::Up {
            return Ok(());
        }
        ensure!(
            self.x.is_finite() && self.y.is_finite(),
            "mouse coordinates must be finite"
        );
        ensure!(
            (1..=2).contains(&self.click_count),
            "clickCount must be 1 or 2"
        );
        ensure!(
            (1..=5_000).contains(&self.press_duration_ms),
            "pressDurationMs must be between 1 and 5000"
        );
        if self.window_id.is_some() {
            self.click_options().validate()?;
        }
        Ok(())
    }

    fn click_options(&self) -> ClickOptions {
        ClickOptions {
            window_id: self.window_id.unwrap_or_default(),
            x: self.x,
            y: self.y,
            expected_bounds: self.expected_bounds,
            move_cursor: true,
            button: self.button,
            click_count: self.click_count,
            press_duration_ms: self.press_duration_ms,
        }
    }
}

/// Owned by the host connection, so worker termination also releases latched buttons.
#[derive(Default)]
pub struct MouseSession {
    #[cfg(target_os = "macos")]
    held: Option<macos::HeldMouse>,
}

impl MouseSession {
    pub fn apply(&mut self, options: &MouseOptions) -> Result<()> {
        let result = options.validate().and_then(|_| {
            #[cfg(target_os = "macos")]
            return macos::mouse(self, options);
            #[cfg(not(target_os = "macos"))]
            anyhow::bail!("mouse input is only supported on macOS");
        });
        if result.is_err() {
            self.release();
        }
        result
    }

    pub fn release(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(held) = self.held.take() {
            held.release();
        }
    }
}

impl Drop for MouseSession {
    fn drop(&mut self) {
        self.release();
    }
}

/// Bounds of the display containing the pointer, in global macOS logical points.
pub fn get_screen_bounds() -> Result<WindowBounds> {
    #[cfg(target_os = "macos")]
    return macos::get_screen_bounds();
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("screen bounds are only supported on macOS");
}

/// Standalone CLI scripts share a session across asynchronous native calls.
type StandaloneMouseRequest = (
    Option<MouseOptions>,
    std::sync::mpsc::SyncSender<Result<()>>,
);
static STANDALONE_MOUSE: std::sync::OnceLock<std::sync::mpsc::Sender<StandaloneMouseRequest>> =
    std::sync::OnceLock::new();

pub fn standalone_mouse(options: &MouseOptions) -> Result<()> {
    let session = STANDALONE_MOUSE.get_or_init(|| {
        let (send, receive) = std::sync::mpsc::channel::<StandaloneMouseRequest>();
        // CGEvents stay on their owning thread, including preconstructed releases.
        std::thread::spawn(move || {
            let mut mouse = MouseSession::default();
            for (options, reply) in receive {
                let result = if let Some(options) = options {
                    mouse.apply(&options)
                } else {
                    mouse.release();
                    Ok(())
                };
                if reply.send(result).is_err() {
                    mouse.release();
                }
            }
        });
        send
    });
    let (reply, result) = std::sync::mpsc::sync_channel(1);
    session
        .send((Some(options.clone()), reply))
        .map_err(|_| anyhow::anyhow!("mouse thread stopped"))?;
    result
        .recv()
        .map_err(|_| anyhow::anyhow!("mouse thread stopped"))?
}

pub fn release_standalone_mouse() {
    if let Some(session) = STANDALONE_MOUSE.get() {
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        if session.send((None, reply)).is_ok() {
            let _ = result.recv();
        }
    }
}

fn default_click_count() -> u8 {
    1
}
fn default_press_duration() -> u64 {
    80
}

impl ClickOptions {
    fn validate(&self) -> Result<()> {
        ensure!(self.window_id != 0, "windowId must be a nonzero window ID");
        ensure!(
            (1..=2).contains(&self.click_count),
            "clickCount must be 1 or 2"
        );
        ensure!(
            (1..=5_000).contains(&self.press_duration_ms),
            "pressDurationMs must be between 1 and 5000"
        );
        ensure!(
            self.x.is_finite() && self.y.is_finite() && self.x >= 0.0 && self.y >= 0.0,
            "click coordinates must be finite nonnegative window-local points"
        );
        if let Some(bounds) = self.expected_bounds {
            ensure!(
                bounds.is_valid(),
                "expectedBounds must contain valid window bounds"
            );
        }
        Ok(())
    }

    fn validate_window(&self, window: &NativeWindow) -> Result<()> {
        self.validate()?;
        ensure!(window.bounds.is_valid(), "target window has invalid bounds");
        ensure!(
            self.x < window.bounds.width && self.y < window.bounds.height,
            "click coordinates are outside the target window"
        );
        if let Some(expected) = self.expected_bounds {
            ensure!(
                expected == window.bounds,
                "target window moved or resized; resolve coordinates again"
            );
        }
        Ok(())
    }
}

/// List the on-screen normal windows of an already-running application. Never launches it.
pub fn get_app_windows(bundle_id: &str) -> Result<Vec<NativeWindow>> {
    ensure!(!bundle_id.trim().is_empty(), "bundleId must not be empty");
    #[cfg(target_os = "macos")]
    return macos::get_app_windows(bundle_id);
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("window-targeted mouse input is only supported on macOS");
}

/// List the on-screen normal windows owned by these processes.
pub fn get_process_windows(pids: &[i32]) -> Result<Vec<NativeWindow>> {
    #[cfg(target_os = "macos")]
    return macos::get_process_windows(pids);
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("window-targeted mouse input is only supported on macOS");
}

/// Send one or two button down/up pairs to one window, off the UI/JS thread.
pub fn click(options: &ClickOptions) -> Result<()> {
    options.validate()?;
    #[cfg(target_os = "macos")]
    return macos::click(options);
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("window-targeted mouse input is only supported on macOS");
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use anyhow::Context;
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSRunningApplication};
    use objc2_core_foundation::{
        CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGPoint,
    };
    use objc2_core_graphics::{
        CGDisplayBounds, CGError, CGEvent, CGEventField, CGEventTapLocation, CGEventType,
        CGGetDisplaysWithPoint, CGMainDisplayID, CGMouseButton, CGPreflightPostEventAccess,
        CGWarpMouseCursorPosition, CGWindowListCopyWindowInfo, CGWindowListOption, kCGNullWindowID,
        kCGWindowBounds, kCGWindowLayer, kCGWindowName, kCGWindowNumber, kCGWindowOwnerPID,
    };
    use objc2_foundation::{NSPoint, NSProcessInfo, NSString};
    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::OnceLock;
    use std::time::Duration;

    type SetWindowLocation = unsafe extern "C" fn(*const CGEvent, CGPoint);

    fn window_location_setter() -> Result<SetWindowLocation> {
        static SETTER: OnceLock<Option<SetWindowLocation>> = OnceLock::new();
        let setter = SETTER.get_or_init(|| {
            // SAFETY: resolve the known Core Graphics SPI ABI; never call a missing symbol.
            let symbol =
                unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"CGEventSetWindowLocation".as_ptr()) };
            (!symbol.is_null())
                .then(|| unsafe { std::mem::transmute::<*mut c_void, SetWindowLocation>(symbol) })
        });
        setter.context("CGEventSetWindowLocation is unavailable on this macOS version")
    }

    fn dictionary_value<'a>(dict: &'a CFDictionary, key: &CFString) -> Option<&'a CFType> {
        // SAFETY: window info is an immutable retained CF dictionary with CF object values.
        let value = unsafe { dict.value((key as *const CFString).cast()) };
        unsafe { value.cast::<CFType>().as_ref() }
    }

    fn number(dict: &CFDictionary, key: &CFString) -> Option<i64> {
        dictionary_value(dict, key)?
            .downcast_ref::<CFNumber>()?
            .as_i64()
    }

    fn windows() -> Result<Vec<NativeWindow>> {
        let list =
            CGWindowListCopyWindowInfo(CGWindowListOption::OptionOnScreenOnly, kCGNullWindowID)
                .context("could not enumerate on-screen windows")?;
        let mut windows = Vec::new();
        for index in 0..list.count() {
            // SAFETY: CGWindowListCopyWindowInfo returns an immutable array of CF dictionaries.
            let entry = unsafe { &*list.value_at_index(index).cast::<CFType>() };
            let Some(dict) = entry.downcast_ref::<CFDictionary>() else {
                continue;
            };
            // SAFETY: these are Core Graphics' exported immutable CFString constants.
            let (id, pid, layer, raw_bounds, title) = unsafe {
                (
                    number(dict, kCGWindowNumber),
                    number(dict, kCGWindowOwnerPID),
                    number(dict, kCGWindowLayer),
                    dictionary_value(dict, kCGWindowBounds),
                    dictionary_value(dict, kCGWindowName),
                )
            };
            if layer != Some(0) {
                continue;
            }
            let (Some(id), Some(pid), Some(raw_bounds)) = (id, pid, raw_bounds) else {
                continue;
            };
            let Some(bounds) = raw_bounds.downcast_ref::<CFDictionary>() else {
                continue;
            };
            let read = |key: &str| -> Option<f64> {
                dictionary_value(bounds, &CFString::from_str(key))?
                    .downcast_ref::<CFNumber>()?
                    .as_f64()
            };
            let (Some(x), Some(y), Some(width), Some(height)) =
                (read("X"), read("Y"), read("Width"), read("Height"))
            else {
                continue;
            };
            let (Ok(window_id), Ok(process_id)) = (u32::try_from(id), i32::try_from(pid)) else {
                continue;
            };
            windows.push(NativeWindow {
                window_id,
                process_id,
                bounds: WindowBounds {
                    x,
                    y,
                    width,
                    height,
                },
                title: title
                    .and_then(|v| v.downcast_ref::<CFString>())
                    .map(ToString::to_string),
            });
        }
        Ok(windows)
    }

    pub(super) fn get_app_windows(bundle_id: &str) -> Result<Vec<NativeWindow>> {
        let pids: Vec<_> = autoreleasepool(|_| {
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
                bundle_id,
            ))
            .iter()
            .map(|app| app.processIdentifier())
            .collect()
        });
        get_process_windows(&pids)
    }

    pub(super) fn get_process_windows(pids: &[i32]) -> Result<Vec<NativeWindow>> {
        autoreleasepool(|_| {
            Ok(windows()?
                .into_iter()
                .filter(|window| pids.contains(&window.process_id))
                .collect())
        })
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> *mut CFType;
        fn AXUIElementCopyAttributeValue(
            element: *const CFType,
            attribute: *const CFString,
            value: *mut *mut CFType,
        ) -> i32;
    }

    fn is_frontmost(pid: i32) -> Result<bool> {
        // AX is a fresh query; NSWorkspace.frontmostApplication can be stale on worker threads.
        // SAFETY: these create/copy APIs transfer owned CF objects, released by CFRetained.
        let application = NonNull::new(unsafe { AXUIElementCreateApplication(pid) })
            .context("could not inspect target application's focus")?;
        let application = unsafe { CFRetained::from_raw(application) };
        let attribute = CFString::from_static_str("AXFrontmost");
        let mut value = std::ptr::null_mut();
        let status =
            unsafe { AXUIElementCopyAttributeValue(&*application, &*attribute, &mut value) };
        // Adopt even on failure so a non-null result is always released.
        let value = NonNull::new(value).map(|raw| unsafe { CFRetained::from_raw(raw) });
        ensure!(
            status == 0,
            "could not read target application's focus (AX error {status})"
        );
        value
            .and_then(|v| v.downcast::<CFBoolean>().ok())
            .map(|v| v.as_bool())
            .context("target application returned an invalid focus value")
    }

    fn mouse_event(
        window: &NativeWindow,
        options: &ClickOptions,
        down: bool,
        click_count: u8,
        background: bool,
        setter: SetWindowLocation,
    ) -> Result<CFRetained<CGEvent>> {
        let (x, y) = (options.x, options.y);
        let local = CGPoint::new(x, y);
        let global = CGPoint::new(window.bounds.x + x, window.bounds.y + y);
        let (event_type, button_number) = match (options.button, down) {
            (MouseButton::Left, true) => (NSEventType::LeftMouseDown, 0),
            (MouseButton::Left, false) => (NSEventType::LeftMouseUp, 0),
            (MouseButton::Right, true) => (NSEventType::RightMouseDown, 1),
            (MouseButton::Right, false) => (NSEventType::RightMouseUp, 1),
            (MouseButton::Middle, true) => (NSEventType::OtherMouseDown, 2),
            (MouseButton::Middle, false) => (NSEventType::OtherMouseUp, 2),
        };
        let flags = if background {
            NSEventModifierFlags::Command
        } else {
            NSEventModifierFlags::empty()
        };
        let ns_event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            event_type, NSPoint::new(x, y), flags, NSProcessInfo::processInfo().systemUptime(),
            window.window_id as isize, None, 1, click_count.into(), if down { 1.0 } else { 0.0 },
        ).context("could not create a window-targeted mouse event")?;
        let source = ns_event
            .CGEvent()
            .context("could not convert the window-targeted mouse event")?;
        let event = CGEvent::new_copy(Some(&source))
            .context("could not copy the window-targeted mouse event")?;
        CGEvent::set_location(Some(&event), global);
        CGEvent::set_integer_value_field(
            Some(&event),
            CGEventField::MouseEventButtonNumber,
            button_number,
        );
        CGEvent::set_integer_value_field(
            Some(&event),
            CGEventField::MouseEventClickState,
            click_count.into(),
        );
        // Subtype 3 and Command on background events are the verified non-activating recipe.
        CGEvent::set_integer_value_field(Some(&event), CGEventField::MouseEventSubtype, 3);
        CGEvent::set_integer_value_field(
            Some(&event),
            CGEventField::MouseEventWindowUnderMousePointer,
            window.window_id.into(),
        );
        CGEvent::set_integer_value_field(
            Some(&event),
            CGEventField::MouseEventWindowUnderMousePointerThatCanHandleThisEvent,
            window.window_id.into(),
        );
        // SAFETY: the SPI was resolved with its actual ABI and `event` is a live mutable copy.
        unsafe { setter(&*event, local) };
        Ok(event)
    }

    pub(super) fn click(options: &ClickOptions) -> Result<()> {
        autoreleasepool(|_| {
            let setter = window_location_setter()?;
            ensure!(
                CGPreflightPostEventAccess(),
                "Accessibility permission is required for window-targeted clicks"
            );
            let window = windows()?
                .into_iter()
                .find(|window| window.window_id == options.window_id)
                .context("target window is no longer on-screen")?;
            options.validate_window(&window)?;
            let background = !is_frontmost(window.process_id)?;
            // Construct every release before posting input so failures cannot leave a button held.
            let events = (1..=options.click_count)
                .map(|count| {
                    Ok((
                        mouse_event(&window, options, true, count, background, setter)?,
                        mouse_event(&window, options, false, count, background, setter)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            if options.move_cursor {
                let point = CGPoint::new(window.bounds.x + options.x, window.bounds.y + options.y);
                let error = CGWarpMouseCursorPosition(point);
                ensure!(
                    error == CGError::Success,
                    "could not move cursor (CG error {error:?})"
                );
            }
            for (index, (down, up)) in events.iter().enumerate() {
                if index > 0 {
                    let current = windows()?
                        .into_iter()
                        .find(|w| w.window_id == options.window_id)
                        .context("target window is no longer on-screen")?;
                    options.validate_window(&current)?;
                    ensure!(
                        current.bounds == window.bounds,
                        "target window moved or resized"
                    );
                }
                post_mouse_event(window.process_id, down);
                std::thread::sleep(Duration::from_millis(options.press_duration_ms));
                post_mouse_event(window.process_id, up);
                if index + 1 < events.len() {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            Ok(())
        })
    }

    fn post_mouse_event(pid: i32, event: &CGEvent) {
        // Releases are constructed before posting, but timestamps must reflect the actual hold.
        CGEvent::set_timestamp(
            Some(event),
            (NSProcessInfo::processInfo().systemUptime() * 1_000_000_000.0) as u64,
        );
        CGEvent::post_to_pid(pid, Some(event));
    }

    pub(super) fn get_screen_bounds() -> Result<WindowBounds> {
        let pointer = CGEvent::new(None).context("could not read cursor position")?;
        let mut display = CGMainDisplayID();
        let mut count = 0;
        // SAFETY: both output pointers refer to live stack values, capacity is one.
        let error = unsafe {
            CGGetDisplaysWithPoint(
                CGEvent::location(Some(&pointer)),
                1,
                &mut display,
                &mut count,
            )
        };
        ensure!(error == CGError::Success, "could not find cursor display");
        if count == 0 {
            display = CGMainDisplayID();
        }
        let frame = CGDisplayBounds(display);
        let bounds = WindowBounds {
            x: frame.origin.x,
            y: frame.origin.y,
            width: frame.size.width,
            height: frame.size.height,
        };
        ensure!(bounds.is_valid(), "display has invalid bounds");
        Ok(bounds)
    }

    pub(super) struct HeldMouse {
        window: Option<NativeWindow>,
        button: MouseButton,
        up: CFRetained<CGEvent>,
    }

    impl HeldMouse {
        pub(super) fn release(self) {
            autoreleasepool(|_| post_input(self.window.as_ref(), &self.up));
        }
    }

    fn post_input(window: Option<&NativeWindow>, event: &CGEvent) {
        if let Some(window) = window {
            post_mouse_event(window.process_id, event);
        } else {
            CGEvent::set_timestamp(
                Some(event),
                (NSProcessInfo::processInfo().systemUptime() * 1_000_000_000.0) as u64,
            );
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
        }
    }

    fn input_event(
        window: Option<&NativeWindow>,
        options: &MouseOptions,
        kind: CGEventType,
        button: MouseButton,
    ) -> Result<CFRetained<CGEvent>> {
        let number = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
        };
        if let Some(window) = window {
            let mut click = options.click_options();
            click.button = button;
            let event = mouse_event(
                window,
                &click,
                kind == event_kind(button, MouseAction::Down),
                options.click_count,
                !is_frontmost(window.process_id)?,
                window_location_setter()?,
            )?;
            CGEvent::set_type(Some(&event), kind);
            CGEvent::set_double_value_field(
                Some(&event),
                CGEventField::MouseEventPressure,
                if matches!(
                    kind,
                    CGEventType::LeftMouseDown
                        | CGEventType::RightMouseDown
                        | CGEventType::OtherMouseDown
                        | CGEventType::LeftMouseDragged
                        | CGEventType::RightMouseDragged
                        | CGEventType::OtherMouseDragged
                ) {
                    1.0
                } else {
                    0.0
                },
            );
            Ok(event)
        } else {
            let event = CGEvent::new_mouse_event(
                None,
                kind,
                CGPoint::new(options.x, options.y),
                CGMouseButton(number),
            )
            .context("could not create global mouse event")?;
            CGEvent::set_integer_value_field(
                Some(&event),
                CGEventField::MouseEventClickState,
                options.click_count.into(),
            );
            Ok(event)
        }
    }

    fn event_kind(button: MouseButton, action: MouseAction) -> CGEventType {
        match (button, action) {
            (MouseButton::Left, MouseAction::Down) => CGEventType::LeftMouseDown,
            (MouseButton::Right, MouseAction::Down) => CGEventType::RightMouseDown,
            (MouseButton::Middle, MouseAction::Down) => CGEventType::OtherMouseDown,
            (MouseButton::Left, MouseAction::Move) => CGEventType::LeftMouseDragged,
            (MouseButton::Right, MouseAction::Move) => CGEventType::RightMouseDragged,
            (MouseButton::Middle, MouseAction::Move) => CGEventType::OtherMouseDragged,
            (MouseButton::Left, _) => CGEventType::LeftMouseUp,
            (MouseButton::Right, _) => CGEventType::RightMouseUp,
            (MouseButton::Middle, _) => CGEventType::OtherMouseUp,
        }
    }

    pub(super) fn mouse(session: &mut MouseSession, options: &MouseOptions) -> Result<()> {
        autoreleasepool(|_| {
            if options.action == MouseAction::Up {
                session.release();
                return Ok(());
            }
            ensure!(
                CGPreflightPostEventAccess(),
                "Accessibility permission is required for mouse input"
            );
            let window = options
                .window_id
                .map(|id| -> Result<_> {
                    let window = windows()?
                        .into_iter()
                        .find(|w| w.window_id == id)
                        .context("target window is no longer on-screen")?;
                    options.click_options().validate_window(&window)?;
                    Ok(window)
                })
                .transpose()?;
            if options.action == MouseAction::Click {
                session.release();
                if window.is_some() {
                    return click(&options.click_options());
                }
                // Prepare all releases before sending any global input.
                let mut clicks = Vec::new();
                for count in 1..=options.click_count {
                    let counted = MouseOptions {
                        click_count: count,
                        ..options.clone()
                    };
                    clicks.push((
                        input_event(
                            None,
                            &counted,
                            event_kind(options.button, MouseAction::Down),
                            options.button,
                        )?,
                        input_event(
                            None,
                            &counted,
                            event_kind(options.button, MouseAction::Up),
                            options.button,
                        )?,
                    ));
                }
                for (index, (down, up)) in clicks.iter().enumerate() {
                    post_input(None, down);
                    std::thread::sleep(Duration::from_millis(options.press_duration_ms));
                    post_input(None, up);
                    if index + 1 < clicks.len() {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
                return Ok(());
            }
            if options.action == MouseAction::Down {
                session.release();
            }
            if let Some(held) = &session.held {
                ensure!(
                    held.window.as_ref().map(|w| w.window_id) == options.window_id,
                    "a drag must remain in its original coordinate scope"
                );
                if let (Some(original), Some(current)) = (&held.window, &window) {
                    ensure!(
                        original.bounds == current.bounds
                            && original.process_id == current.process_id,
                        "target window moved or resized"
                    );
                }
            }
            let button = session
                .held
                .as_ref()
                .map_or(options.button, |held| held.button);
            let dragging = options.action == MouseAction::Move && session.held.is_some();
            let kind = if options.action == MouseAction::Move && !dragging {
                CGEventType::MouseMoved
            } else {
                event_kind(button, options.action)
            };
            let event = input_event(window.as_ref(), options, kind, button)?;
            let up = if options.action == MouseAction::Down || dragging {
                Some(input_event(
                    window.as_ref(),
                    options,
                    event_kind(button, MouseAction::Up),
                    button,
                )?)
            } else {
                None
            };
            if let Some(window) = &window {
                let error = CGWarpMouseCursorPosition(CGPoint::new(
                    window.bounds.x + options.x,
                    window.bounds.y + options.y,
                ));
                ensure!(
                    error == CGError::Success,
                    "could not move cursor (CG error {error:?})"
                );
            }
            if options.action == MouseAction::Down {
                session.held = Some(HeldMouse {
                    window: window.clone(),
                    button,
                    up: up.unwrap(),
                });
            } else if let (Some(held), Some(up)) = (&mut session.held, up) {
                held.up = up;
            }
            post_input(window.as_ref(), &event);
            Ok(())
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn constructs_global_drag_events_with_signed_coordinates_without_posting() {
            let options: MouseOptions =
                serde_json::from_str(r#"{"action":"move","x":-420.5,"y":-10.25}"#).unwrap();
            for (button, number, down, drag, up) in [
                (
                    MouseButton::Left,
                    0,
                    CGEventType::LeftMouseDown,
                    CGEventType::LeftMouseDragged,
                    CGEventType::LeftMouseUp,
                ),
                (
                    MouseButton::Right,
                    1,
                    CGEventType::RightMouseDown,
                    CGEventType::RightMouseDragged,
                    CGEventType::RightMouseUp,
                ),
                (
                    MouseButton::Middle,
                    2,
                    CGEventType::OtherMouseDown,
                    CGEventType::OtherMouseDragged,
                    CGEventType::OtherMouseUp,
                ),
            ] {
                for (action, expected) in [
                    (MouseAction::Down, down),
                    (MouseAction::Move, drag),
                    (MouseAction::Up, up),
                ] {
                    let kind = event_kind(button, action);
                    assert_eq!(kind, expected);
                    let event = input_event(None, &options, kind, button).unwrap();
                    assert_eq!(CGEvent::r#type(Some(&event)), expected);
                    assert_eq!(
                        CGEvent::location(Some(&event)),
                        CGPoint::new(-420.5, -10.25)
                    );
                    assert_eq!(
                        CGEvent::integer_value_field(
                            Some(&event),
                            CGEventField::MouseEventButtonNumber
                        ),
                        number
                    );
                }
            }
            let event =
                input_event(None, &options, CGEventType::MouseMoved, MouseButton::Left).unwrap();
            assert_eq!(CGEvent::r#type(Some(&event)), CGEventType::MouseMoved);
        }

        #[test]
        fn constructs_window_targeted_events_without_posting_input() {
            type GetWindowLocation = unsafe extern "C" fn(*const CGEvent) -> CGPoint;
            // Read the stamped field directly: NSEvent's locationInWindow falls back to
            // screen coordinates when our deliberately fake window ID has no real NSWindow.
            let getter =
                unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"CGEventGetWindowLocation".as_ptr()) };
            assert!(!getter.is_null());
            let getter = unsafe { std::mem::transmute::<*mut c_void, GetWindowLocation>(getter) };
            let window = NativeWindow {
                window_id: 123,
                process_id: 456,
                bounds: WindowBounds {
                    x: -500.0,
                    y: 200.0,
                    width: 300.0,
                    height: 600.0,
                },
                title: None,
            };
            let setter = window_location_setter().unwrap();
            for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
                for count in [1, 2] {
                    for background in [false, true] {
                        for down in [false, true] {
                            let options = ClickOptions {
                                window_id: 123,
                                x: 30.5,
                                y: 40.25,
                                expected_bounds: None,
                                move_cursor: false,
                                button,
                                click_count: count,
                                press_duration_ms: 80,
                            };
                            let event =
                                mouse_event(&window, &options, down, count, background, setter)
                                    .unwrap();
                            let decoded = NSEvent::eventWithCGEvent(&event).unwrap();
                            assert_eq!(decoded.windowNumber(), 123);
                            assert_eq!(unsafe { getter(&*event) }, CGPoint::new(30.5, 40.25));
                            assert_eq!(
                                CGEvent::location(Some(&event)),
                                CGPoint::new(-469.5, 240.25)
                            );
                            assert_eq!(
                                CGEvent::integer_value_field(
                                    Some(&event),
                                    CGEventField::MouseEventSubtype
                                ),
                                3
                            );
                            for field in [CGEventField::MouseEventWindowUnderMousePointer, CGEventField::MouseEventWindowUnderMousePointerThatCanHandleThisEvent] {
                                assert_eq!(CGEvent::integer_value_field(Some(&event), field), 123);
                            }
                            assert_eq!(
                                decoded
                                    .modifierFlags()
                                    .contains(NSEventModifierFlags::Command),
                                background
                            );
                            let (event_type, number) = match (button, down) {
                                (MouseButton::Left, true) => (NSEventType::LeftMouseDown, 0),
                                (MouseButton::Left, false) => (NSEventType::LeftMouseUp, 0),
                                (MouseButton::Right, true) => (NSEventType::RightMouseDown, 1),
                                (MouseButton::Right, false) => (NSEventType::RightMouseUp, 1),
                                (MouseButton::Middle, true) => (NSEventType::OtherMouseDown, 2),
                                (MouseButton::Middle, false) => (NSEventType::OtherMouseUp, 2),
                            };
                            assert_eq!(decoded.r#type(), event_type);
                            assert_eq!(decoded.buttonNumber(), number);
                            assert_eq!(decoded.clickCount(), isize::from(count));
                            assert_eq!(
                                CGEvent::integer_value_field(
                                    Some(&event),
                                    CGEventField::MouseEventButtonNumber
                                ),
                                number as i64
                            );
                            assert_eq!(
                                CGEvent::integer_value_field(
                                    Some(&event),
                                    CGEventField::MouseEventClickState
                                ),
                                i64::from(count)
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> NativeWindow {
        NativeWindow {
            window_id: 123,
            process_id: 456,
            bounds: WindowBounds {
                x: -500.0,
                y: 200.0,
                width: 300.0,
                height: 600.0,
            },
            title: None,
        }
    }

    #[test]
    fn mouse_options_distinguish_global_points_window_offsets_and_cleanup() {
        let global: MouseOptions =
            serde_json::from_str(r#"{"action":"move","x":-500,"y":-20}"#).unwrap();
        assert!(global.validate().is_ok());
        assert!(
            MouseOptions {
                window_id: Some(123),
                ..global.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            MouseOptions {
                x: f64::NAN,
                ..global.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            MouseOptions {
                action: MouseAction::Up,
                x: f64::NAN,
                window_id: Some(0),
                ..global.clone()
            }
            .validate()
            .is_ok()
        );
        assert!(serde_json::from_str::<MouseOptions>(r#"{"action":"drag"}"#).is_err());
        assert!(
            serde_json::from_str::<MouseOptions>(r#"{"action":"down","button":"unknown"}"#)
                .is_err()
        );
        let local: MouseOptions = serde_json::from_str(
            r#"{"action":"down","windowId":123,"x":30,"y":20,"button":"middle"}"#,
        )
        .unwrap();
        assert!(local.click_options().validate_window(&window()).is_ok());
        assert_eq!(local.click_options().button, MouseButton::Middle);
    }

    #[test]
    fn defaults_to_one_left_click_and_validates_button_count_and_hold_duration() {
        let options: ClickOptions =
            serde_json::from_str(r#"{"windowId":123,"x":30,"y":20}"#).unwrap();
        assert_eq!(options.button, MouseButton::Left);
        assert_eq!(options.click_count, 1);
        assert_eq!(options.press_duration_ms, 80);
        assert!(!options.move_cursor);
        let visible: ClickOptions =
            serde_json::from_str(r#"{"windowId":123,"x":30,"y":20,"moveCursor":true}"#).unwrap();
        assert!(visible.move_cursor);
        assert!(visible.validate_window(&window()).is_ok());
        for (button, count, hold) in [
            (MouseButton::Right, 1, 80),
            (MouseButton::Middle, 1, 80),
            (MouseButton::Left, 2, 80),
            (MouseButton::Left, 1, 500),
        ] {
            assert!(
                ClickOptions {
                    button,
                    click_count: count,
                    press_duration_ms: hold,
                    ..options.clone()
                }
                .validate_window(&window())
                .is_ok()
            );
        }
        for (count, hold) in [(0, 80), (3, 80), (1, 0), (1, 5001)] {
            assert!(
                ClickOptions {
                    click_count: count,
                    press_duration_ms: hold,
                    ..options.clone()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            serde_json::from_str::<ClickOptions>(
                r#"{"windowId":123,"x":30,"y":20,"button":"unknown"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn validates_coordinates_before_any_native_input() {
        let window = window();
        for (x, y) in [(0.0, 0.0), (299.5, 599.5)] {
            assert!(
                ClickOptions {
                    window_id: 123,
                    button: MouseButton::Left,
                    click_count: 1,
                    press_duration_ms: 80,
                    move_cursor: true,
                    x,
                    y,
                    expected_bounds: Some(window.bounds)
                }
                .validate_window(&window)
                .is_ok()
            );
        }
        for (x, y) in [
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
            (-1.0, 0.0),
            (300.0, 0.0),
            (0.0, 600.0),
        ] {
            assert!(
                ClickOptions {
                    window_id: 123,
                    button: MouseButton::Left,
                    click_count: 1,
                    press_duration_ms: 80,
                    move_cursor: true,
                    x,
                    y,
                    expected_bounds: None
                }
                .validate_window(&window)
                .is_err()
            );
        }
        assert!(
            click(&ClickOptions {
                window_id: 0,
                button: MouseButton::Left,
                click_count: 1,
                press_duration_ms: 80,
                move_cursor: true,
                x: 0.0,
                y: 0.0,
                expected_bounds: None
            })
            .is_err()
        );
    }

    #[test]
    fn rejects_moved_or_resized_windows() {
        let window = window();
        for bounds in [
            WindowBounds {
                x: -499.0,
                ..window.bounds
            },
            WindowBounds {
                height: 601.0,
                ..window.bounds
            },
        ] {
            assert!(
                ClickOptions {
                    window_id: 123,
                    button: MouseButton::Left,
                    click_count: 1,
                    press_duration_ms: 80,
                    move_cursor: true,
                    x: 1.0,
                    y: 1.0,
                    expected_bounds: Some(bounds)
                }
                .validate_window(&window)
                .is_err()
            );
        }
    }
}
