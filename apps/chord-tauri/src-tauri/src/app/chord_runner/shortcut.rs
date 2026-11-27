use crate::app::chord_runner::ChordActionTask;
#[cfg(target_os = "macos")]
use crate::app::chord_runner::media_key::{self, MediaKey};
use crate::models::{ShortcutChordAction, SimulatedShortcutAction};
use anyhow::{Context, Result, anyhow};
use nject::injectable;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;

#[injectable]
#[derive(Clone)]
pub struct ShortcutChordActionTaskRunner {
    handle: AppHandle,
}

impl ShortcutChordActionTaskRunner {
    // We use `rdev` for simulate instead of Enigo because rdev sends actual keypresses
    // instead of enigo's input injection (this works better in some apps, e.g. cmd+1 in IntelliJ)
    #[cfg(target_os = "macos")]
    pub fn simulate_shortcut_actions(&self, actions: Vec<SimulatedShortcutAction>) -> Result<()> {
        /// One step of the shortcut. Media keys are not key codes on macOS (see `media_key`):
        /// they are pressed whole when the shortcut presses them, and their release is skipped.
        enum Step {
            Key(rdev::EventType, bool),
            Media(MediaKey),
        }
        let steps: Vec<Step> = actions
            .into_iter()
            .filter_map(|action| -> Option<Result<Step>> {
                let (key, pressed, suppress_shift) = match action {
                    SimulatedShortcutAction::Press(key, suppress_shift) => {
                        (key, true, suppress_shift)
                    }
                    SimulatedShortcutAction::Release(key, suppress_shift) => {
                        (key, false, suppress_shift)
                    }
                };
                if let Some(media) = MediaKey::from_code(key.0) {
                    return pressed.then_some(Ok(Step::Media(media)));
                }
                Some(key.try_into().map(|key| {
                    let event = if pressed {
                        rdev::EventType::KeyPress(key)
                    } else {
                        rdev::EventType::KeyRelease(key)
                    };
                    Step::Key(event, suppress_shift)
                }))
            })
            .collect::<Result<_>>()?;

        // rdev must be run on the main thread. Wait for the closure so failures can
        // propagate to the task runner and trigger user feedback.
        let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
        self.handle.run_on_main_thread(move || {
            let mut result = Ok(());
            for step in steps {
                let outcome = match step {
                    Step::Key(event, suppress_shift) => rdev::simulate(&event, suppress_shift)
                        .map_err(|e| anyhow!("error simulating keypress: {e}")),
                    Step::Media(key) => media_key::press(key)
                        .map_err(|e| anyhow!("error pressing media key {key:?}: {e:#}")),
                };
                if let Err(error) = outcome {
                    result = Err(error);
                    break;
                }
            }
            let _ = result_tx.send(result);
        })?;

        result_rx
            .recv()
            .context("main thread stopped before simulating shortcut")?
    }

    #[cfg(not(target_os = "macos"))]
    pub fn simulate_shortcut_actions(&self, actions: Vec<SimulatedShortcutAction>) -> Result<()> {
        crate::desktop_platform::simulate_keys(&actions)
    }

    pub fn get_start_simulated_shortcut_actions(
        &self,
        action: &ShortcutChordAction,
        num_times: u32,
    ) -> Vec<SimulatedShortcutAction> {
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

    pub fn get_end_simulated_shortcut_actions(
        &self,
        action: &ShortcutChordAction,
    ) -> Vec<SimulatedShortcutAction> {
        let suppress_shift = !action.simulated_shortcut.has_shift();
        action
            .simulated_shortcut
            .chords
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

#[derive(Debug)]
pub struct ShortcutChordActionTaskRun {
    join_handle: Option<JoinHandle<Result<()>>>,
    end_simulated_shortcut_actions: Vec<SimulatedShortcutAction>,
}

async fn repeat_shortcut_actions(
    count: u32,
    start: Vec<SimulatedShortcutAction>,
    end: Vec<SimulatedShortcutAction>,
    mut simulate: impl FnMut(Vec<SimulatedShortcutAction>) -> Result<()>,
) -> Result<()> {
    for index in 0..count {
        simulate(start.clone())?;
        if index + 1 < count {
            simulate(end.clone())?;
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

impl ShortcutChordActionTaskRunner {
    pub fn start(
        &self,
        task: &ChordActionTask,
        action: &ShortcutChordAction,
    ) -> Result<ShortcutChordActionTaskRun> {
        let start_actions = self.get_start_simulated_shortcut_actions(action, 1);
        let end_actions = self.get_end_simulated_shortcut_actions(action);
        let join_handle = if task.num_times > 1 {
            let runner = self.clone();
            let end_actions = end_actions.clone();
            let num_times = task.num_times;
            Some(tauri::async_runtime::spawn(repeat_shortcut_actions(
                num_times,
                start_actions,
                end_actions,
                move |actions| runner.simulate_shortcut_actions(actions),
            )))
        } else {
            self.simulate_shortcut_actions(start_actions)?;
            None
        };

        Ok(ShortcutChordActionTaskRun {
            join_handle,
            end_simulated_shortcut_actions: end_actions,
        })
    }

    pub async fn end(&self, task_run: &mut ShortcutChordActionTaskRun) -> Result<()> {
        let result = if let Some(join_handle) = task_run.join_handle.as_mut() {
            match join_handle.await {
                Ok(result) => result,
                Err(error) => Err(error.into()),
            }
        } else {
            Ok(())
        };
        self.simulate_shortcut_actions(std::mem::take(
            &mut task_run.end_simulated_shortcut_actions,
        ))?;
        result
    }

    pub fn abort(&self, task_run: ShortcutChordActionTaskRun) -> Result<()> {
        if let Some(join_handle) = task_run.join_handle {
            join_handle.abort();
        }
        self.simulate_shortcut_actions(task_run.end_simulated_shortcut_actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Key;
    use keycode::KeyMappingCode;

    #[tokio::test]
    async fn finite_shortcut_repetition_releases_between_runs_and_holds_only_the_last() {
        let key = Key(KeyMappingCode::KeyA);
        let mut events = Vec::new();
        repeat_shortcut_actions(
            3,
            vec![SimulatedShortcutAction::Press(key, false)],
            vec![SimulatedShortcutAction::Release(key, false)],
            |actions| {
                events.extend(actions.into_iter().map(|action| match action {
                    SimulatedShortcutAction::Press(_, _) => "press",
                    SimulatedShortcutAction::Release(_, _) => "release",
                }));
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!(events, ["press", "release", "press", "release", "press"]);
    }

    #[tokio::test]
    async fn failed_shortcuts_do_not_continue_the_remaining_repetitions() {
        let mut calls = 0;
        let error = repeat_shortcut_actions(u32::MAX, vec![], vec![], |_| {
            calls += 1;
            anyhow::bail!("simulation failed")
        })
        .await
        .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(error.to_string(), "simulation failed");
    }
}
