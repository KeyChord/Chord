use crate::IndicatorPanel;
use anyhow::Result;
use objc2_app_kit::NSWindowAnimationBehavior;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, WebviewWindow};
use tauri_nspanel::{CollectionBehavior, Panel, PanelLevel, StyleMask, WebviewWindowExt};

const INDICATOR_WIDTH: u32 = 640;
const INDICATOR_HEIGHT: u32 = 180;

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

    fn show(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Showing chorder panel");
        let panel = self.panel.clone();
        let is_visible = self.is_visible.clone();

        handle.clone().run_on_main_thread(move || {
            let native_panel = panel.as_panel();
            native_panel.setContentSize(tauri_nspanel::objc2_foundation::NSSize::new(
                INDICATOR_WIDTH as f64,
                INDICATOR_HEIGHT as f64,
            ));

            if let Some(screen) = native_panel.screen() {
                let visible_frame = screen.visibleFrame();
                let x = visible_frame.origin.x
                    + ((visible_frame.size.width - INDICATOR_WIDTH as f64) / 2.0).max(0.0);
                let y = visible_frame.origin.y
                    + ((visible_frame.size.height - INDICATOR_HEIGHT as f64) / 2.0).max(0.0);
                native_panel.setFrameOrigin(tauri_nspanel::objc2_foundation::NSPoint::new(x, y));
            }

            panel.show();
            panel.order_front_regardless();
            is_visible.store(true, Ordering::Relaxed);
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

    pub fn ensure_hidden(&self, handle: AppHandle) -> Result<()> {
        if self.is_visible() {
            self.hide(handle)?;
        }

        Ok(())
    }

    pub fn ensure_visible(&self, handle: AppHandle) -> Result<()> {
        if !self.is_visible() {
            self.show(handle)?;
        }

        Ok(())
    }
}
