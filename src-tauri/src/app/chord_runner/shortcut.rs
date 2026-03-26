use crate::app::SafeAppHandle;
use crate::input::Key;
use anyhow::Result;
use keycode::KeyMappingCode;
use serde::Serialize;
use std::str::FromStr;
use typeshare::typeshare;

#[derive(Clone)]
pub struct ChordShortcutRunner {
    handle: SafeAppHandle,
}

impl ChordShortcutRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle }
    }

    pub fn press_shortcut(&self, shortcut: Shortcut, num_times: usize) -> Result<()> {
        self.apply_actions(shortcut.to_press_actions(num_times))?;
        Ok(())
    }

    pub fn release_shortcut(&self, shortcut: Shortcut) -> Result<()> {
        self.apply_actions(shortcut.to_release_actions())?;
        Ok(())
    }

    // We use `rdev` for simulate instead of Enigo because rdev sends actual keypresses
    // instead of enigo's input injection (this works better in some apps, e.g. cmd+1 in IntelliJ)
    fn apply_actions(&self, actions: Vec<ShortcutAction>) -> Result<()> {
        let events: Vec<(rdev::EventType, bool)> = actions
            .into_iter()
            .map(|action| -> Result<(rdev::EventType, bool)> {
                Ok(match action {
                    ShortcutAction::Press(key, suppress_shift) => {
                        (rdev::EventType::KeyPress(key.try_into()?), suppress_shift)
                    }
                    ShortcutAction::Release(key, suppress_shift) => {
                        (rdev::EventType::KeyRelease(key.try_into()?), suppress_shift)
                    }
                })
            })
            .collect::<Result<_>>()?;

        // rdev must be run on main thread
        self.handle.run_on_main_thread(move || {
            for (event, suppress_shift) in events {
                if let Err(e) = rdev::simulate(&event, suppress_shift) {
                    log::error!("error simulating {} keypress", e);
                }
            }
        })?;

        Ok(())
    }
}

/// Represents a parsed keyboard shortcut, e.g. "cmd+shift+n".
#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct Shortcut {
    pub chords: Vec<ShortcutChord>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ShortcutChord {
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    Press(Key, bool),
    Release(Key, bool),
}

impl Shortcut {
    pub fn parse(shortcut_str: &str) -> Result<Self> {
        let mut chords = Vec::new();
        for chord in shortcut_str.split(' ') {
            let mut keys = Vec::new();
            for key_name in chord.split('+') {
                if let Ok(key) = Key::from_str(key_name) {
                    keys.push(key);
                } else {
                    anyhow::bail!("Failed to parse shortcut {}", shortcut_str);
                }
            }
            chords.push(ShortcutChord { keys });
        }

        Ok(Shortcut { chords })
    }

    fn has_shift(&self) -> bool {
        let has_shift = self.chords.iter().any(|chord| {
            chord.keys.iter().any(|key| {
                matches!(
                    key,
                    Key(KeyMappingCode::ShiftLeft) | Key(KeyMappingCode::ShiftRight)
                )
            })
        });

        has_shift
    }

    fn to_press_actions(&self, num_times: usize) -> Vec<ShortcutAction> {
        let mut actions = Vec::new();
        let suppress_shift = !self.has_shift();

        for i in 0..num_times {
            for (index, chord) in self.chords.iter().enumerate() {
                for &key in &chord.keys {
                    actions.push(ShortcutAction::Press(key, suppress_shift));
                }

                let is_last_chord = index + 1 == self.chords.len();
                let is_last_iteration = i + 1 == num_times;

                // Only release if NOT the final chord of the final iteration
                if !(is_last_chord && is_last_iteration) {
                    for &key in chord.keys.iter().rev() {
                        actions.push(ShortcutAction::Release(key, suppress_shift));
                    }
                }
            }
        }

        actions
    }

    fn to_release_actions(&self) -> Vec<ShortcutAction> {
        let suppress_shift = !self.has_shift();
        self.chords
            .last()
            .into_iter()
            .flat_map(|chord| {
                chord
                    .keys
                    .iter()
                    .rev()
                    .copied()
                    .map(|k| ShortcutAction::Release(k, suppress_shift))
            })
            .collect()
    }
}
