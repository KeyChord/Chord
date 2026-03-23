use crate::IndicatorPanel;
use anyhow::Result;
use objc2_app_kit::{
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSView,
    NSWindowAnimationBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{NSInteger, MainThreadMarker, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, WebviewWindow};
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
    pub is_visible: Arc<AtomicBool>,
    pub panel: Arc<dyn Panel>,
    pub window: WebviewWindow,
}

impl ChorderIndicatorUi {
    pub fn from_window(window: WebviewWindow) -> Result<Self> {
        let _ = window.set_ignore_cursor_events(true);

        let panel = window.to_panel::<IndicatorPanel>()?;
        panel.set_level(PanelLevel::ScreenSaver.into());
        panel.set_has_shadow(false);
        panel.set_opaque(false);
        panel.set_transparent(true);
        panel.set_ignores_mouse_events(true);
        panel.set_becomes_key_only_if_needed(false);
        panel.set_style_mask(StyleMask::empty().borderless().nonactivating_panel().into());
        panel.set_floating_panel(true);
        panel.set_hides_on_deactivate(false);
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
        handle: AppHandle,
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

    pub fn configure_surface(&self, handle: AppHandle, rect: NativeSurfaceRect) -> Result<()> {
        Self::configure_window_surface(&self.window, handle, rect)
    }

    fn show(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Showing chorder panel");
        let panel = self.panel.clone();
        let is_visible = self.is_visible.clone();

        handle.clone().run_on_main_thread(move || {
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
            panel.show();
            panel.order_front_regardless();
            is_visible.store(true, Ordering::Relaxed);
        })?;

        Ok(())
    }

    pub fn reveal(&self, handle: AppHandle) -> Result<()> {
        let panel = self.panel.clone();

        handle.run_on_main_thread(move || {
            panel.set_alpha_value(1.0);
        })?;

        Ok(())
    }

    fn hide(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Hiding chorder panel");
        let is_visible = self.is_visible.clone();
        let panel = self.panel.clone();

        handle.clone().run_on_main_thread(move || {
            panel.hide();
            is_visible.store(false, Ordering::Relaxed);
        })?;

        Ok(())
    }

    pub fn ensure_hidden(&self, handle: AppHandle) -> Result<bool> {
        if self.is_visible() {
            self.hide(handle)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn ensure_visible(&self, handle: AppHandle) -> Result<bool> {
        if !self.is_visible() {
            self.show(handle)?;
            return Ok(true);
        }

        Ok(false)
    }
}
