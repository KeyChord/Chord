use crate::IndicatorPanel;
use crate::models::ChordLayout;
use crate::startup::APP_STATE_STORE_PATH;
use crate::state::{ChordPanelObservable, ChordPanelState, Observable};
use anyhow::{Result, ensure};
use arc_swap::ArcSwap;
use nject::injectable;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSScreen, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSWindowAnimationBehavior, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSInteger, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindow, Wry};
use tauri_nspanel::{CollectionBehavior, PanelBuilder, PanelHandle, PanelLevel, StyleMask};
use tauri_plugin_store::StoreExt;
use window_vibrancy::NSVisualEffectViewTagged;

const INDICATOR_WIDTH: f64 = 640.0;
const FLOATING_BOTTOM_MARGIN: f64 = 24.0;
const LAYOUT_STORE_KEY: &str = "chordLayout";
const NATIVE_SURFACE_TAG: NSInteger = 91376255;
/// How long a show waits for the page to draw the current state before revealing anyway.
const SURFACE_READY_TIMEOUT: Duration = Duration::from_millis(160);
/// Shows in a row the page has not answered before Chord reloads it. A page that misses
/// one is usually just slow to wake; one that misses several is stuck.
const UNANSWERED_SHOWS_BEFORE_RELOAD: u32 = 3;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSurfaceRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub flush_left: bool,
}

/// The window the page's card needs in a layout, in points: in the sidebar, the width
/// from the screen's left edge, and floating, the height from the bottom margin, each
/// including the card's shadow.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct PanelExtent {
    pub layout: ChordLayout,
    pub width: f64,
    pub height: f64,
}

#[injectable]
pub struct ChordModePanel {
    #[inject(ArcSwap::new(Arc::new(None)))]
    panel: ArcSwap<Option<PanelHandle<Wry>>>,

    /// What the page last said its card needs. The window is sized to it, since WebKit
    /// keeps pixels for the whole window, including its transparent parts.
    #[inject(parking_lot::Mutex::new(None))]
    extent: parking_lot::Mutex<Option<PanelExtent>>,

    /// Shows in a row whose page did not report back in time (see
    /// `UNANSWERED_SHOWS_BEFORE_RELOAD`).
    #[inject(AtomicU32::new(0))]
    unanswered_shows: AtomicU32,

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
        ensure!(
            native_panel.ignoresMouseEvents() && !native_panel.acceptsMouseMovedEvents(),
            "chord-mode panel must pass all mouse events through to underlying windows"
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
        let store = self.handle.store(APP_STATE_STORE_PATH)?;
        let layout = store
            .get(LAYOUT_STORE_KEY)
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        self.observable
            .set_state(|prev| ChordPanelState { layout, ..prev })?;
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
        // Hiding Chord (⌘H, or handing focus back when Settings closes) hides every one of
        // its windows that can hide, and a hidden panel stays hidden when ordered front.
        panel.as_panel().setCanHide(false);
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

    pub fn set_layout(&self, layout: ChordLayout) -> Result<()> {
        let store = self.handle.store(APP_STATE_STORE_PATH)?;
        let previous = store.get(LAYOUT_STORE_KEY);
        store.set(LAYOUT_STORE_KEY, serde_json::to_value(layout)?);
        if let Err(error) = store.save() {
            if let Some(previous) = previous {
                store.set(LAYOUT_STORE_KEY, previous);
            } else {
                store.delete(LAYOUT_STORE_KEY);
            }
            return Err(error.into());
        }
        self.observable
            .set_state(|prev| ChordPanelState { layout, ..prev })?;
        let panel = self.panel.load_full();
        let extent = *self.extent.lock();
        self.run_on_main_thread_sync(move || {
            if let Some(panel) = panel.as_ref() {
                Self::position_panel(panel, layout, extent);
            }
            Ok(())
        })?;
        self.emit_will_show()
    }

    pub fn cycle_layout(&self) -> Result<()> {
        self.set_layout(self.observable.get_state()?.layout.next())
    }

    /// Each layout keeps the card anchored to the window's edge it grows away from: the
    /// sidebar card to the left, the floating card to the bottom. So the window only
    /// narrows to the sidebar card, keeping the height the page centers it in, and only
    /// lowers to the floating card, keeping the width the page centers it in. Without an
    /// extent for the layout, it spans the most the layout uses.
    fn frame_for_layout(
        layout: ChordLayout,
        visible_frame: NSRect,
        extent: Option<PanelExtent>,
    ) -> NSRect {
        let extent = extent.filter(|extent| extent.layout == layout);
        match layout {
            ChordLayout::SidebarLeft => {
                let widest = INDICATOR_WIDTH.min(visible_frame.size.width);
                let width = extent.map_or(widest, |extent| extent.width.min(widest).max(0.0));
                NSRect::new(
                    visible_frame.origin,
                    NSSize::new(width, visible_frame.size.height),
                )
            }
            ChordLayout::FloatingBottom => {
                let margin = FLOATING_BOTTOM_MARGIN.min(visible_frame.size.height);
                let tallest = visible_frame.size.height - margin;
                let height = extent.map_or(tallest, |extent| extent.height.min(tallest).max(0.0));
                NSRect::new(
                    NSPoint::new(visible_frame.origin.x, visible_frame.origin.y + margin),
                    NSSize::new(visible_frame.size.width, height),
                )
            }
        }
    }

    /// Sizes the window to what the page says its card needs.
    pub fn fit_to_card(&self, extent: PanelExtent) -> Result<()> {
        {
            let mut current = self.extent.lock();
            if *current == Some(extent) {
                return Ok(());
            }
            *current = Some(extent);
        }
        let layout = self.observable.get_state()?.layout;
        let panel = self.panel.load_full();
        self.handle.run_on_main_thread(move || {
            if let Some(panel) = panel.as_ref() {
                Self::position_panel(panel, layout, Some(extent));
            }
        })?;
        Ok(())
    }

    /// The display the panel shows on: the one it was last on, as before. When that
    /// display is gone, `NSWindow.screen` is nil, which left the panel off screen while
    /// chords still ran, so the main display takes its place.
    fn target_screen(panel: &PanelHandle<Wry>) -> Option<Retained<NSScreen>> {
        let mtm = MainThreadMarker::new()?;
        panel
            .as_panel()
            .screen()
            .or_else(|| NSScreen::mainScreen(mtm))
            .or_else(|| NSScreen::screens(mtm).firstObject())
    }

    fn position_panel(panel: &PanelHandle<Wry>, layout: ChordLayout, extent: Option<PanelExtent>) {
        let native_panel = panel.as_panel();
        // Keep the native window out of mouse hit testing after layout changes
        // and before every show, including its transparent webview and vibrancy.
        native_panel.setIgnoresMouseEvents(true);
        native_panel.setAcceptsMouseMovedEvents(false);
        match Self::target_screen(panel) {
            Some(screen) => {
                let frame = Self::frame_for_layout(layout, screen.visibleFrame(), extent);
                native_panel.setContentSize(frame.size);
                native_panel.setFrameOrigin(frame.origin);
            }
            None => log::warn!("No display to show the Hint Panel on"),
        }
    }

    fn emit_will_show(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.emit("chorder-will-show", ())?;
        Ok(())
    }

    /// Asks the page to draw the current state and waits briefly for it to say it has.
    /// Must not run on the main thread, which delivers the page's answer.
    pub fn prepare_surface_before_reveal(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        if MainThreadMarker::new().is_some() {
            // Waiting here would block the answer it waits for.
            return self.emit_will_show();
        }
        let (tx, rx) = mpsc::sync_channel(1);
        let listener = window.once("chorder-surface-ready", move |_| {
            let _ = tx.send(());
        });
        self.emit_will_show()?;
        match rx.recv_timeout(SURFACE_READY_TIMEOUT) {
            Ok(()) => {
                self.unanswered_shows.store(0, Ordering::SeqCst);
                Ok(())
            }
            Err(error) => {
                window.unlisten(listener);
                self.on_show_unanswered(&window);
                Err(error.into())
            }
        }
    }

    /// The page did not answer a show. WebKit can leave a page that was hidden for a long
    /// while unable to draw, so after a few misses in a row it is reloaded; the page
    /// reads every state again when it starts.
    fn on_show_unanswered(&self, window: &WebviewWindow) {
        let misses = self.unanswered_shows.fetch_add(1, Ordering::SeqCst) + 1;
        log::warn!("The Hint Panel did not draw in time ({misses} in a row)");
        if misses < UNANSWERED_SHOWS_BEFORE_RELOAD {
            return;
        }
        self.unanswered_shows.store(0, Ordering::SeqCst);
        log::warn!("Reloading the Hint Panel's page");
        if let Err(error) = window.reload() {
            log::error!("Failed to reload the Hint Panel: {error:#}");
        }
    }

    pub fn preload(&self) -> Result<()> {
        log::info!("Preloading chorder panel");

        if self.ensure_visible(true)? {
            // Whatever happens while it draws, the panel must not stay ordered in.
            let prepared = self.prepare_surface_before_reveal();
            self.ensure_hidden()?;
            prepared?;
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
            .url(WebviewUrl::App("panel.html".into()))
            .title("Chords")
            .no_activate(true)
            .ignores_mouse_events(true)
            .accepts_mouse_moved_events(false)
            .movable_by_window_background(false)
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
                    // Since macOS 14, WebKit suspends a hidden web view's page. The panel
                    // is hidden between chords, so every show had to wake it first, and a
                    // page that woke late (or not at all) left the panel blank while the
                    // chord still ran.
                    .background_throttling(BackgroundThrottlingPolicy::Disabled)
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

            // Only the edge-attached sidebar extends past its left corners.
            let left_extension = if rect.flush_left { rect.radius } else { 0.0 };
            let frame = NSRect::new(
                NSPoint::new(rect.x - left_extension, rect.y),
                NSSize::new(rect.width + left_extension, rect.height),
            );
            let vibrancy_view = NSVisualEffectViewTagged::initWithFrame(
                // Guaranteed by `run_on_main_thread`.
                MainThreadMarker::new()
                    .expect("run_on_main_thread closure not on main thread")
                    .alloc(),
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
        let layout = self.observable.get_state()?.layout;
        let extent = *self.extent.lock();

        self.run_on_main_thread_sync(move || {
            let panel = panel
                .as_ref()
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("chord-mode panel was not initialized"))?;
            Self::position_panel(panel, layout, extent);
            // The panel is only drawn: it takes no clicks and no keys. A broken invariant
            // is put right and logged rather than leaving chord mode without its panel.
            if let Err(error) = Self::ensure_nonactivating(panel) {
                log::warn!("Repairing the Hint Panel before showing it: {error:#}");
                panel.set_style_mask(Self::nonactivating_style_mask().into());
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
            if let Err(error) = Self::ensure_nonactivating(panel) {
                log::error!("The Hint Panel is showing in a state it should not be in: {error:#}");
            }
            Ok(())
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

    /// Orders the panel's window in. `is_hint_panel_visible` is whether the Hint Panel
    /// is drawn in it; a hidden one is a `toggle` away.
    pub fn ensure_visible(&self, is_hint_panel_visible: bool) -> Result<bool> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: is_hint_panel_visible,
            ..prev
        })?;
        self.show()?;
        Ok(true)
    }

    /// Toggle frontend visibility without changing the native window ordering.
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
    fn layouts_follow_the_displays_visible_frame_including_negative_origins() {
        let screen = NSRect::new(NSPoint::new(-1600.0, 80.0), NSSize::new(1440.0, 900.0));
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, screen, None),
            NSRect::new(NSPoint::new(-1600.0, 80.0), NSSize::new(640.0, 900.0))
        );
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::FloatingBottom, screen, None),
            NSRect::new(NSPoint::new(-1600.0, 104.0), NSSize::new(1440.0, 876.0))
        );
        let small = NSRect::new(NSPoint::new(0.0, -100.0), NSSize::new(320.0, 100.0));
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, small, None),
            small
        );
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::FloatingBottom, small, None),
            NSRect::new(NSPoint::new(0.0, -76.0), NSSize::new(320.0, 76.0))
        );
        let tiny = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(320.0, 10.0));
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::FloatingBottom, tiny, None),
            NSRect::new(NSPoint::new(0.0, 10.0), NSSize::new(320.0, 0.0))
        );
    }

    #[test]
    fn the_window_fits_the_card_along_the_edge_it_grows_from() {
        let screen = NSRect::new(NSPoint::new(-1600.0, 80.0), NSSize::new(1440.0, 900.0));
        let card = |layout, width, height| {
            Some(PanelExtent {
                layout,
                width,
                height,
            })
        };
        let sidebar = card(ChordLayout::SidebarLeft, 412.0, 900.0);
        let floating = card(ChordLayout::FloatingBottom, 1440.0, 140.0);
        // The sidebar narrows, keeping the height the card is centered in.
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, screen, sidebar),
            NSRect::new(NSPoint::new(-1600.0, 80.0), NSSize::new(412.0, 900.0))
        );
        // The floating panel lowers its top, keeping its bottom and the width it centers in.
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::FloatingBottom, screen, floating),
            NSRect::new(NSPoint::new(-1600.0, 104.0), NSSize::new(1440.0, 140.0))
        );
        // A card bigger than the layout allows gets what the layout allows.
        assert_eq!(
            ChordModePanel::frame_for_layout(
                ChordLayout::SidebarLeft,
                screen,
                card(ChordLayout::SidebarLeft, 2000.0, 900.0)
            ),
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, screen, None)
        );
        // An extent from the other layout, from before a switch, isn't used.
        assert_eq!(
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, screen, floating),
            ChordModePanel::frame_for_layout(ChordLayout::SidebarLeft, screen, None)
        );
        // A display with no room left gets an empty window, not a panic.
        let tiny = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(320.0, 10.0));
        assert_eq!(
            ChordModePanel::frame_for_layout(
                ChordLayout::FloatingBottom,
                tiny,
                card(ChordLayout::FloatingBottom, 320.0, 140.0)
            ),
            NSRect::new(NSPoint::new(0.0, 10.0), NSSize::new(320.0, 0.0))
        );
    }

    #[test]
    fn chord_mode_style_is_borderless_and_nonactivating() {
        assert_eq!(
            ChordModePanel::nonactivating_style_mask().value(),
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel
        );
    }
}
