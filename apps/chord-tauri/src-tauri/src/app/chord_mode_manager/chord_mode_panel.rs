use crate::IndicatorPanel;
use crate::state::{ChordPanelObservable, ChordPanelState, Observable};
use anyhow::{Result, ensure};
use arc_swap::ArcSwap;
use nject::injectable;
use objc2_app_kit::{
    NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSWindowAnimationBehavior, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSInteger, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindow, Wry};
use tauri_nspanel::{CollectionBehavior, PanelBuilder, PanelHandle, PanelLevel, StyleMask};
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

#[injectable]
pub struct ChordModePanel {
    #[inject(ArcSwap::new(Arc::new(None)))]
    panel: ArcSwap<Option<PanelHandle<Wry>>>,

    handle: AppHandle,
    observable: ChordPanelObservable,
}

impl ChordModePanel {
    fn nonactivating_style_mask() -> StyleMask {
        StyleMask::empty().borderless().nonactivating_panel()
    }

    fn ensure_nonactivating(panel: &PanelHandle<Wry>) -> Result<()> {
        let native_panel = panel.as_panel();
        ensure!(
            !panel.can_become_key_window(),
            "chord-mode panel must never become the key window"
        );
        ensure!(
            !panel.can_become_main_window(),
            "chord-mode panel must never become the main window"
        );
        ensure!(
            native_panel
                .styleMask()
                .contains(NSWindowStyleMask::NonactivatingPanel),
            "chord-mode panel is missing NSWindowStyleMaskNonactivatingPanel"
        );
        ensure!(
            !native_panel.isKeyWindow() && !native_panel.isMainWindow(),
            "chord-mode panel unexpectedly became key or main"
        );
        Ok(())
    }

    fn run_on_main_thread_sync<F>(&self, operation: F) -> Result<()>
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        if MainThreadMarker::new().is_some() {
            return operation();
        }

        let (tx, rx) = mpsc::sync_channel(1);
        self.handle.run_on_main_thread(move || {
            let _ = tx.send(operation());
        })?;
        rx.recv()
            .map_err(|_| anyhow::anyhow!("main-thread panel operation was cancelled"))?
    }

    pub fn init(&self) -> Result<()> {
        self.get_or_create_window()?;
        let panel = self.panel.load_full();
        let panel = panel
            .as_ref()
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("chord-mode panel was not created"))?;
        panel.set_level(PanelLevel::ScreenSaver.into());
        panel.set_has_shadow(false);
        panel.set_opaque(false);
        panel.set_transparent(true);
        panel.set_ignores_mouse_events(true);
        panel.set_accepts_mouse_moved_events(false);
        panel.set_movable_by_window_background(false);
        panel.set_works_when_modal(false);
        panel.set_becomes_key_only_if_needed(false);
        panel.set_style_mask(Self::nonactivating_style_mask().into());
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
        Self::ensure_nonactivating(panel)?;
        Ok(())
    }

    fn emit_will_show(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.emit("chorder-will-show", ())?;
        Ok(())
    }

    pub fn prepare_surface_before_reveal(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        let (tx, rx) = mpsc::sync_channel(1);
        window.once("chorder-surface-ready", move |_| {
            let _ = tx.send(());
        });
        self.emit_will_show()?;
        rx.recv_timeout(Duration::from_millis(160))?;
        Ok(())
    }

    pub fn preload(&self) -> Result<()> {
        log::info!("Preloading chorder panel");

        if self.ensure_visible()? {
            self.prepare_surface_before_reveal()?;
            self.ensure_hidden()?;
        }

        Ok(())
    }

    pub fn get_or_create_window(&self) -> Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("chords") {
            return Ok(window);
        }

        ensure!(
            MainThreadMarker::new().is_some(),
            "chord-mode panel must be created on the main thread"
        );

        // PanelBuilder creates a regular NSWindow before converting it to NSPanel.
        // Guard that short creation phase so even the intermediate window cannot
        // activate Chord or replace the user's frontmost application.
        let panel = PanelBuilder::<Wry, IndicatorPanel>::new(&self.handle, "chords")
            .url(WebviewUrl::App("index.html".into()))
            .title("Chords")
            .no_activate(true)
            .style_mask(Self::nonactivating_style_mask())
            .with_window(|window| {
                window
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
            })
            .build()?;

        Self::ensure_nonactivating(&panel)?;
        self.panel.store(Arc::new(Some(panel)));

        self.handle
            .get_webview_window("chords")
            .ok_or_else(|| anyhow::anyhow!("chord-mode webview window was not registered"))
    }

    fn webview_ns_view(window: &WebviewWindow) -> Result<usize> {
        let handle = window.window_handle()?;
        match handle.as_raw() {
            RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
            _ => anyhow::bail!("unsupported platform for native surface vibrancy"),
        }
    }

    pub fn toggle_inspector(&self) -> Result<bool> {
        let window = self.get_or_create_window()?;
        if window.is_visible()? {
            window.hide()?;
            #[cfg(debug_assertions)]
            window.close_devtools();
            Ok(false)
        } else {
            window.show()?;
            window.unminimize()?;
            window.set_focus()?;
            #[cfg(debug_assertions)]
            window.open_devtools();
            Ok(true)
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

    fn show(&self) -> Result<()> {
        let panel = self.panel.load_full();

        self.run_on_main_thread_sync(move || {
            let panel = panel
                .as_ref()
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("chord-mode panel was not initialized"))?;
            Self::ensure_nonactivating(panel)?;

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

            // This orders the nonactivating panel above inactive applications
            // without asking AppKit to activate Chord or make the panel key.
            panel.order_front_regardless();

            let _ = panel.make_first_responder(None);
            panel.resign_key_window();
            panel.resign_main_window();
            Self::ensure_nonactivating(panel)
        })
    }

    pub fn reveal(&self) -> Result<()> {
        let panel = self.panel.load_full();
        self.handle.run_on_main_thread(move || {
            if let Some(panel) = panel.as_ref() {
                panel.set_alpha_value(1.0);
            }
        })?;

        Ok(())
    }

    fn hide(&self) -> Result<()> {
        let panel = self.panel.load_full();

        self.handle.clone().run_on_main_thread(move || {
            if let Some(panel) = panel.as_ref() {
                panel.hide();
            }
        })?;

        Ok(())
    }

    pub fn ensure_hidden(&self) -> Result<bool> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: false,
            ..prev
        })?;
        self.hide()?;
        Ok(true)
    }

    pub fn ensure_visible(&self) -> Result<bool> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: true,
            ..prev
        })?;
        self.show()?;
        Ok(true)
    }

    /// This only changes the state so the frontend gets a chance to animate it.
    pub fn toggle(&self) -> Result<()> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: !prev.is_visible,
            ..prev
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_mode_style_is_borderless_and_nonactivating() {
        assert_eq!(
            ChordModePanel::nonactivating_style_mask().value(),
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel
        );
    }
}
