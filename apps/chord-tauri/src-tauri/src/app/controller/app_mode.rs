use crate::app::AppHandleExt;
use crate::models::{CapturedKeyEvent, Key, KeyEvent};
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


    pub fn handle_key_event(&self, input: &CapturedKeyEvent) -> Result<()> {
        let key_event = &input.event;
        let app_mode = self.app_mode();
        log::debug!("handling key event in {app_mode} mode");

        match app_mode {
            AppMode::Chord => {
                let chord_mode_manager = self.handle.app_state().chord_mode_manager();
                if chord_mode_manager.is_inserting() {
                    // Insert mode keeps Chord active without Space, and no key reaches
                    // other apps. Caps+Space closes the quote and Esc cancels Chord.
                    if input.activates_chord_mode() {
                        chord_mode_manager.finish_insert()?;
                        self.handle.app_state().keyboard().on_normal_mode_started();
                    } else if key_event == &KeyEvent::Press(Key(KeyMappingCode::Escape)) {
                        self.set_idle_mode()?;
                    } else {
                        chord_mode_manager.handle_insert_key_event(key_event)?;
                    }
                    return Ok(());
                }
                chord_mode_manager.handle_key_event(&key_event);

                if key_event == &KeyEvent::Release(Key(KeyMappingCode::Space)) {
                    self.set_idle_mode();
                }
            }
            AppMode::Idle => {
                if input.activates_chord_mode() {
                    self.set_chord_mode()?;
                    self.handle.app_state().keyboard().on_normal_mode_started();
                }
            }
        }

        Ok(())
    }

    pub fn set_idle_mode(&self) -> Result<()> {
        self.mode.store(AppMode::Idle, Ordering::SeqCst);
        // First, and whatever fails below: the keyboard and Caps Lock go back to how
        // they were. Hiding the panel can take a while, and what is typed meanwhile
        // is no longer Chord's.
        self.handle.app_state().keyboard().on_chord_mode_ended();
        self.observable.set_state(|_| AppModeState::Idle)?;
        let chord_mode_manager = self.handle.app_state().chord_mode_manager();
        chord_mode_manager.ensure_inactive()?;
        Ok(())
    }

    pub fn set_chord_mode(&self) -> Result<()> {
        self.mode.store(AppMode::Chord, Ordering::SeqCst);
        self.observable.set_state(|_| AppModeState::Chord)?;
        let chord_mode_manager = self.handle.app_state().chord_mode_manager();
        chord_mode_manager.ensure_active()?;
        Ok(())
    }
}
