use crate::models::ChordLayout;
use crate::startup::APP_STATE_STORE_PATH;
use crate::state::{ChordPanelObservable, ChordPanelState, Observable};
use anyhow::{Context, Result};
use nject::injectable;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

const LAYOUT_STORE_KEY: &str = "chordLayout";

#[injectable]
pub struct ChordModePanel {
    handle: AppHandle,
    observable: ChordPanelObservable,
}

impl ChordModePanel {
    pub fn init(&self) -> Result<()> {
        let layout = self
            .handle
            .store(APP_STATE_STORE_PATH)?
            .get(LAYOUT_STORE_KEY)
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        self.observable
            .set_state(|prev| ChordPanelState { layout, ..prev })?;
        self.get_or_create_window()?;
        Ok(())
    }

    pub fn get_or_create_window(&self) -> Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("chords") {
            return Ok(window);
        }
        let window =
            WebviewWindowBuilder::new(&self.handle, "chords", WebviewUrl::App("panel.html".into()))
                .title("Chords")
                .visible(false)
                .focused(false)
                .focusable(false)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .build()?;
        #[cfg(windows)]
        window.set_ignore_cursor_events(true)?;
        Ok(window)
    }

    pub fn preload(&self) -> Result<()> {
        self.prepare_surface_before_reveal()
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
        self.position()?;
        self.prepare_surface_before_reveal()
    }

    pub fn cycle_layout(&self) -> Result<()> {
        self.set_layout(self.observable.get_state()?.layout.next())
    }

    fn position(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        let cursor = window.cursor_position()?;
        let monitor = window
            .monitor_from_point(cursor.x, cursor.y)?
            .or(window.primary_monitor()?);
        let Some(monitor) = monitor else {
            return Ok(());
        };
        let area = monitor.work_area();
        let sidebar = (640.0 * monitor.scale_factor()).round() as u32;
        let width = match self.observable.get_state()?.layout {
            ChordLayout::SidebarLeft => sidebar.min(area.size.width),
            ChordLayout::FloatingBottom => area.size.width,
        };
        window.set_position(PhysicalPosition::new(area.position.x, area.position.y))?;
        window.set_size(PhysicalSize::new(width, area.size.height))?;
        Ok(())
    }

    pub fn prepare_surface_before_reveal(&self) -> Result<()> {
        self.get_or_create_window()?.emit("chorder-will-show", ())?;
        Ok(())
    }

    pub fn reveal(&self) -> Result<()> {
        let window = self.get_or_create_window()?;
        window.show()?;
        // Tao needs the GTK window to be realized before shaping its input region.
        #[cfg(target_os = "linux")]
        window.set_ignore_cursor_events(true)?;
        Ok(())
    }

    pub fn ensure_hidden(&self) -> Result<bool> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: false,
            ..prev
        })?;
        self.get_or_create_window()?.hide()?;
        Ok(true)
    }

    pub fn ensure_visible(&self, is_visible: bool) -> Result<bool> {
        self.observable
            .set_state(|prev| ChordPanelState { is_visible, ..prev })?;
        self.position()?;
        Ok(true)
    }

    pub fn toggle(&self) -> Result<()> {
        self.observable.set_state(|prev| ChordPanelState {
            is_visible: !prev.is_visible,
            ..prev
        })?;
        Ok(())
    }

    pub fn toggle_inspector(&self) -> Result<bool> {
        #[cfg(debug_assertions)]
        {
            let window = self
                .handle
                .get_webview_window("chords")
                .context("Chords window is unavailable")?;
            if window.is_devtools_open() {
                window.close_devtools();
                return Ok(false);
            }
            window.open_devtools();
            Ok(true)
        }
        #[cfg(not(debug_assertions))]
        Ok(false)
    }
}
