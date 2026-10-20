//! The window a chord run targets. Chord reads it once when a completed input starts
//! running, so every repetition and loop iteration acts on the same window even after
//! the run itself (or the user) moves focus elsewhere.

use tauri::AppHandle;

/// The focused, else main, window of `app_id`, or of the focused application when absent,
/// as a Core Graphics window number. `None` when no window can be identified.
#[cfg(target_os = "macos")]
pub fn focused_window_id(handle: &AppHandle, app_id: Option<&str>) -> Option<u32> {
    macos::focused_window_id(handle, app_id)
}

#[cfg(not(target_os = "macos"))]
pub fn focused_window_id(_handle: &AppHandle, _app_id: Option<&str>) -> Option<u32> {
    None
}

#[cfg(target_os = "macos")]
mod macos {
    use crate::app::desktop_app::running_apps_with_id;
    use objc2::rc::autoreleasepool;
    use objc2_application_services::{AXError, AXUIElement};
    use objc2_core_foundation::{CFRetained, CFString, CFType, ConcreteType};
    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::OnceLock;
    use tauri::AppHandle;

    /// A busy application may still answer; one that takes longer has no target window.
    const MESSAGING_TIMEOUT_SECONDS: f32 = 1.0;

    type GetWindow = unsafe extern "C" fn(*const AXUIElement, *mut u32) -> AXError;

    fn window_number_getter() -> Option<GetWindow> {
        static GETTER: OnceLock<Option<GetWindow>> = OnceLock::new();
        *GETTER.get_or_init(|| {
            // SAFETY: resolve the known HIServices SPI ABI; never call a missing symbol.
            let symbol =
                unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"_AXUIElementGetWindow".as_ptr()) };
            (!symbol.is_null())
                .then(|| unsafe { std::mem::transmute::<*mut c_void, GetWindow>(symbol) })
        })
    }

    fn attribute<T: ConcreteType>(
        element: &AXUIElement,
        name: &'static str,
    ) -> Option<CFRetained<T>> {
        let mut raw: *const CFType = std::ptr::null();
        // SAFETY: the out parameter is live storage; copy transfers an owned CF value.
        let error = unsafe {
            element.copy_attribute_value(&CFString::from_static_str(name), NonNull::from(&mut raw))
        };
        let value =
            NonNull::new(raw.cast_mut()).map(|pointer| unsafe { CFRetained::from_raw(pointer) });
        if error == AXError::Success {
            value?.downcast::<T>().ok()
        } else {
            None
        }
    }

    fn application(handle: &AppHandle, app_id: Option<&str>) -> Option<CFRetained<AXUIElement>> {
        let application = match app_id {
            // Window handlers resolve an app ID to its first running instance as well.
            Some(app_id) => {
                let pid = running_apps_with_id(handle, app_id)
                    .first()?
                    .processIdentifier();
                unsafe { AXUIElement::new_application(pid) }
            }
            // AX is a fresh query; NSWorkspace.frontmostApplication can be stale off the
            // main thread.
            None => {
                let system = unsafe { AXUIElement::new_system_wide() };
                unsafe { system.set_messaging_timeout(MESSAGING_TIMEOUT_SECONDS) };
                attribute::<AXUIElement>(&system, "AXFocusedApplication")?
            }
        };
        unsafe { application.set_messaging_timeout(MESSAGING_TIMEOUT_SECONDS) };
        Some(application)
    }

    pub(super) fn focused_window_id(handle: &AppHandle, app_id: Option<&str>) -> Option<u32> {
        let get_window = window_number_getter()?;
        autoreleasepool(|_| {
            let application = application(handle, app_id)?;
            let window = attribute::<AXUIElement>(&application, "AXFocusedWindow")
                .or_else(|| attribute(&application, "AXMainWindow"))?;
            let mut id = 0;
            // SAFETY: the window element is retained for the call; `id` is live storage.
            let status = unsafe { get_window(&*window, &mut id) };
            (status == AXError::Success && id != 0).then_some(id)
        })
    }
}
