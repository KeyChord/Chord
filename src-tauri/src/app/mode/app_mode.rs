use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use std::sync::atomic::{AtomicBool, Ordering};
use atomic_enum::atomic_enum;
use anyhow::Result;
use tauri::AppHandle;
use crate::app::AppHandleExt;
use crate::models::{Key, KeyEvent};
use crate::state::{AppModeObservable, AppModeState, Observable};

#[derive(PartialEq)]
#[atomic_enum(AtomicAppModeState)]
pub(super) enum AppMode {
    Idle,
    Chord,
}

impl AppMode {
    pub fn is_idle(&self) -> bool {
        self == &AppMode::Idle
    }

    pub fn is_chord(&self) -> bool {
        self == &AppMode::Chord
    }
}

pub struct AppModeManager {
    pub(super) mode: AtomicAppModeState,
    pub(super) observable: AppModeObservable,

    pub(super)handle: AppHandle
}

impl AppModeManager {
    pub fn mode(&self) -> AppMode {
        self.mode.load(Ordering::SeqCst)
    }

    pub fn set_idle_mode(&self) -> Result<()> {
        self.mode.store(AppMode::Idle, Ordering::SeqCst);
        self.observable.set_state(|_| AppModeState::Idle)?;
        Ok(())
    }

    pub fn set_chord_mode(&self) -> Result<()> {
        self.mode.store(AppMode::Chord, Ordering::SeqCst);
        self.observable.set_state(|_| AppModeState::Chord)?;
        Ok(())
    }
}
