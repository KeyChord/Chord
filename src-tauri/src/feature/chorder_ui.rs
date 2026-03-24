use crate::IndicatorPanel;
use crate::feature::SafeAppHandle;
use anyhow::Result;
use objc2::msg_send;
use objc2_app_kit::{
    NSRunningApplication, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSWindowAnimationBehavior, NSWindowOrderingMode, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSInteger, NSPoint, NSRect, NSSize, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{WebviewUrl, WebviewWindow};
use tauri_nspanel::{CollectionBehavior, Panel, PanelLevel, StyleMask, WebviewWindowExt};
use window_vibrancy::NSVisualEffectViewTagged;

const INDICATOR_WIDTH: u32 = 640;
const NATIVE_SURFACE_TAG: NSInteger = 91376255;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSurfaceRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

#[derive(Clone)]
pub struct ChorderIndicatorUi {
    pub handle: SafeAppHandle,
    pub is_visible: Arc<AtomicBool>,
    pub panel: Arc<dyn Panel>,
    pub window: WebviewWindow,
}

impl ChorderIndicatorUi {
    fn restore_frontmost_app(bundle_id: &str, pid: i32) {
        let bundle_id = NSString::from_str(bundle_id);
        let running_apps =
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);
        let app = running_apps
            .iter()
            .find(|app| app.processIdentifier() == pid)
            .or_else(|| running_apps.iter().next());

        let Some(app) = app else {
            return;
        };

        unsafe {
            let _: bool = msg_send![&**app, activateWithOptions: 0usize];
        }
    }

    pub fn new(handle: SafeAppHandle) -> Result<Self> {
        let window = handle
            .new_webview_window_builder("chords", WebviewUrl::App("index.html".into()))?
            .title("Chords")
            .inner_size(640.0, 180.0)
            .visible(false)
            .focused(false)
            .focusable(false)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .visible_on_all_workspaces(true)
            .shadow(false)
            .accept_first_mouse(false)
            .build()?;

        let _ = window.set_focusable(false);
        let _ = window.set_ignore_cursor_events(true);

        let panel = window.to_panel::<IndicatorPanel>()?;
        panel.set_level(PanelLevel::ScreenSaver.into());
        panel.set_has_shadow(false);
        panel.set_opaque(false);
        panel.set_transparent(true);
        panel.set_ignores_mouse_events(true);
        panel.set_accepts_mouse_moved_events(false);
        panel.set_movable_by_window_background(false);
        panel.set_works_when_modal(false);
        panel.set_becomes_key_only_if_needed(false);
        panel.set_style_mask(StyleMask::empty().borderless().nonactivating_panel().into());
        panel.set_floating_panel(true);
        panel.set_hides_on_deactivate(false);
        let _ = panel.make_first_responder(None);
        panel.resign_key_window();
        panel.resign_main_window();
        panel
            .as_panel()
            .setAnimationBehavior(NSWindowAnimationBehavior::None);
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .stationary()
                .full_screen_auxiliary()
                .ignores_cycle()
                .into(),
        );

        Ok(Self {
            handle,
            is_visible: Arc::new(AtomicBool::new(false)),
            window,
            panel,
        })
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.load(Ordering::Relaxed)
    }

    fn webview_ns_view(window: &WebviewWindow) -> Result<usize> {
        let handle = window.window_handle()?;
        match handle.as_raw() {
            RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
            _ => anyhow::bail!("unsupported platform for native surface vibrancy"),
        }
    }

    pub fn configure_window_surface(
        window: &WebviewWindow,
        handle: SafeAppHandle,
        rect: NativeSurfaceRect,
    ) -> Result<()> {
        let ns_view = Self::webview_ns_view(window)?;

        handle.run_on_main_thread(move || unsafe {
            let view: &NSView = &*(ns_view as *mut NSView);
            if let Some(existing_view) = view.viewWithTag(NATIVE_SURFACE_TAG) {
                existing_view.removeFromSuperview();
            }

            let frame = NSRect::new(
                NSPoint::new(rect.x - rect.radius, rect.y),
                NSSize::new(rect.width + rect.radius, rect.height),
            );
            let vibrancy_view = NSVisualEffectViewTagged::initWithFrame(
                MainThreadMarker::new().unwrap().alloc(),
                frame,
                NATIVE_SURFACE_TAG,
            );
            vibrancy_view.setMaterial(NSVisualEffectMaterial::HUDWindow);
            vibrancy_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            vibrancy_view.setState(NSVisualEffectState::Active);
            vibrancy_view.setCornerRadius(rect.radius);

            view.addSubview_positioned_relativeTo(
                &vibrancy_view,
                NSWindowOrderingMode::Below,
                None,
            );
        })?;

        Ok(())
    }

    fn show(&self) -> Result<()> {
        log::debug!("Showing chorder panel");
        let panel = self.panel.clone();
        let is_visible = self.is_visible.clone();

        self.handle.clone().run_on_main_thread(move || {
            let current_pid = std::process::id() as i32;
            let previous_frontmost = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .and_then(|app| {
                    let pid = app.processIdentifier();
                    let bundle_id = app.bundleIdentifier()?.to_string();
                    (pid > 0 && pid != current_pid).then_some((bundle_id, pid))
                });
            let native_panel = panel.as_panel();
            if let Some(screen) = native_panel.screen() {
                let visible_frame = screen.visibleFrame();
                native_panel.setContentSize(tauri_nspanel::objc2_foundation::NSSize::new(
                    INDICATOR_WIDTH as f64,
                    visible_frame.size.height,
                ));

                let x = visible_frame.origin.x;
                let y = visible_frame.origin.y;
                native_panel.setFrameOrigin(tauri_nspanel::objc2_foundation::NSPoint::new(x, y));
            }

            panel.set_alpha_value(0.0);
            let _ = panel.make_first_responder(None);
            panel.resign_key_window();
            panel.resign_main_window();

            // Order the window without ever requesting key or main-window status.
            unsafe {
                let _: () = msg_send![native_panel, orderFront: objc2::ffi::nil];
            }

            let _ = panel.make_first_responder(None);
            panel.resign_key_window();
            panel.resign_main_window();

            // NSPanel's non-activating flags are not always sufficient when the
            // underlying webview window is reordered. If AppKit still activates
            // Chord, explicitly hand focus back to the previously frontmost app.
            let chord_stole_activation = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .is_some_and(|app| app.processIdentifier() == current_pid);
            if chord_stole_activation {
                if let Some((bundle_id, pid)) = previous_frontmost {
                    Self::restore_frontmost_app(&bundle_id, pid);
                }
            }
            is_visible.store(true, Ordering::Relaxed);
        })?;

        Ok(())
    }

    pub fn reveal(&self) -> Result<()> {
        let panel = self.panel.clone();

        self.handle.run_on_main_thread(move || {
            panel.set_alpha_value(1.0);
        })?;

        Ok(())
    }

    fn hide(&self) -> Result<()> {
        log::debug!("Hiding chorder panel");
        let is_visible = self.is_visible.clone();
        let panel = self.panel.clone();

        self.handle.clone().run_on_main_thread(move || {
            panel.hide();
            is_visible.store(false, Ordering::Relaxed);
        })?;

        Ok(())
    }

    pub fn ensure_hidden(&self) -> Result<bool> {
        if self.is_visible() {
            self.hide()?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn ensure_visible(&self) -> Result<bool> {
        if !self.is_visible() {
            self.show()?;
            return Ok(true);
        }

        Ok(false)
    }
}
