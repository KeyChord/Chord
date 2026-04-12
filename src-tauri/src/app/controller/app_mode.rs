use crate::app::AppHandleExt;
use crate::models::{Key, KeyEvent};
use crate::state::{AppModeObservable, AppModeState, Observable};
use anyhow::Result;
use atomic_enum::atomic_enum;
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use nject::{inject, injectable};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};
use derive_more::Display;

#[atomic_enum(AtomicAppModeState)]
#[derive(PartialEq, Display)]
pub enum AppMode {
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
pub struct AppController {
    #[inject(AtomicAppModeState::new(AppMode::Idle))]
    mode: AtomicAppModeState,

    handle: AppHandle,
    observable: AppModeObservable,
}

impl AppController {
    pub fn app_mode(&self) -> AppMode {
        self.mode.load(Ordering::SeqCst)
    }


    pub fn handle_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        let app_mode = self.app_mode();
        log::debug!("handling key event in {app_mode} mode");

        match app_mode {
            AppMode::Chord => {
                let chord_mode_manager = self.handle.app_state().chord_mode_manager();
                chord_mode_manager.handle_key_event(&key_event);

                if key_event == &KeyEvent::Release(Key(KeyMappingCode::Space)) {
                    self.set_idle_mode();
                }
            }
            AppMode::Idle => {
                let Some(keyboard_state) = self.handle.app_state().keyboard().state() else {
                    return Ok(())
                };

                if key_event == &KeyEvent::Press(Key(KeyMappingCode::Space)) &&
                    keyboard_state.is_caps_pressed() {
                    self.set_chord_mode();
                }
            }
        }

        Ok(())
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
