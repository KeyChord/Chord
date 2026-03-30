use crate::input::Key;
use keycode::KeyMappingCode;
use serde::Serialize;
use std::str::FromStr;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct SimulatedShortcut {
    pub chords: Vec<SimulatedShortcutChord>,
}

impl SimulatedShortcut {
    pub fn has_shift(&self) -> bool {
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
}

impl FromStr for SimulatedShortcut {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chords = Vec::new();
        for chord in s.split(' ') {
            let mut keys = Vec::new();
            for key_name in chord.split('+') {
                if let Ok(key) = Key::from_str(key_name) {
                    keys.push(key);
                } else {
                    anyhow::bail!("Failed to parse shortcut {}", s);
                }
            }
            chords.push(SimulatedShortcutChord { keys });
        }

        Ok(Self { chords })
    }
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct SimulatedShortcutChord {
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulatedShortcutAction {
    Press(Key, bool),
    Release(Key, bool),
}
