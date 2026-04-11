use crate::app::AppHandleExt;
use crate::models::{Key, KeyEvent};
use crate::state::{AppModeObservable, AppModeState, Observable};
use anyhow::Result;
use atomic_enum::atomic_enum;
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::{inject, injectable};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

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

#[injectable]
pub struct AppModeManager {
    #[inject(AtomicAppModeState::new(AppMode::Idle))]
    mode: AtomicAppModeState,

    handle: AppHandle,
    observable: AppModeObservable,
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
