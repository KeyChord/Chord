use crate::app::SafeAppHandle;
use crate::input::Key;
use anyhow::Result;
use keycode::KeyMappingCode;
use std::str::FromStr;
use crate::models::{ChordShortcutAction, SimulatedShortcutAction};

pub struct ChordShortcutActionRunner {
    handle: SafeAppHandle,
}

impl ChordShortcutActionRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        Self { handle }
    }

    // TODO: a runner shouldn't know how many times it's being run, and should handle internal state
    pub fn start(&self, action: &ChordShortcutAction, num_times: u32) -> Result<()> {
        self.simulate_shortcut_actions(self.get_start_simulated_shortcut_actions(action, num_times))?;
        Ok(())
    }

    pub fn end(&self, action: &ChordShortcutAction) -> Result<()> {
        self.simulate_shortcut_actions(self.get_end_simulated_shortcut_actions(action))?;
        Ok(())
    }

    // We use `rdev` for simulate instead of Enigo because rdev sends actual keypresses
    // instead of enigo's input injection (this works better in some apps, e.g. cmd+1 in IntelliJ)
    fn simulate_shortcut_actions(&self, actions: Vec<SimulatedShortcutAction>) -> Result<()> {
        let events: Vec<(rdev::EventType, bool)> = actions
            .into_iter()
            .map(|action| -> Result<(rdev::EventType, bool)> {
                Ok(match action {
                    SimulatedShortcutAction::Press(key, suppress_shift) => {
                        (rdev::EventType::KeyPress(key.try_into()?), suppress_shift)
                    }
                    SimulatedShortcutAction::Release(key, suppress_shift) => {
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

    fn get_start_simulated_shortcut_actions(&self, action: &ChordShortcutAction, num_times: u32) -> Vec<SimulatedShortcutAction> {
        let mut actions = Vec::new();
        let suppress_shift = !action.simulated_shortcut.has_shift();

        for i in 0..num_times {
            for (index, chord) in action.simulated_shortcut.chords.iter().enumerate() {
                for &key in &chord.keys {
                    actions.push(SimulatedShortcutAction::Press(key, suppress_shift));
                }

                let is_last_chord = index + 1 == action.simulated_shortcut.chords.len();
                let is_last_iteration = i + 1 == num_times;

                // Only release if NOT the final chord of the final iteration
                if !(is_last_chord && is_last_iteration) {
                    for &key in chord.keys.iter().rev() {
                        actions.push(SimulatedShortcutAction::Release(key, suppress_shift));
                    }
                }
            }
        }

        actions
    }

    fn get_end_simulated_shortcut_actions(&self, action: &ChordShortcutAction) -> Vec<SimulatedShortcutAction> {
        let suppress_shift = !action.simulated_shortcut.has_shift();
        action.simulated_shortcut.chords
            .last()
            .into_iter()
            .flat_map(|chord| {
                chord
                    .keys
                    .iter()
                    .rev()
                    .copied()
                    .map(|k| SimulatedShortcutAction::Release(k, suppress_shift))
            })
            .collect()
    }
}
