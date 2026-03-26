use self::javascript::ChordJavascriptRunner;
use self::runtime::ChordPayload;
use self::runtime::ChordRuntime;
use self::shell::ChordShellRunner;
use self::shortcut::ChordShortcutRunner;
use crate::app::SafeAppHandle;
use crate::app::chord_package::Chord;
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Notify;

pub mod javascript;
pub mod registry;
pub mod runtime;
pub mod shell;
pub mod shortcut;

#[derive(Clone)]
enum QueuedChordAction {
    Press {
        runtime: Arc<ChordRuntime>,
        chord_payload: ChordPayload,
    },
    Release {
        chord: Chord,
    },
}

// TODO: registry should be part of ChordRunner
pub struct ChordRunner {
    pub shortcut: ChordShortcutRunner,
    queue: Arc<Mutex<VecDeque<QueuedChordAction>>>,
    queue_signal: Arc<Notify>,

    _handle: SafeAppHandle,
}

impl ChordRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        let shortcut = ChordShortcutRunner::new(handle.clone());
        let shell = ChordShellRunner::new(handle.clone());
        let javascript = ChordJavascriptRunner::new(handle.clone());
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue_signal = Arc::new(Notify::new());

        tauri::async_runtime::spawn(Self::process_queue(
            queue.clone(),
            queue_signal.clone(),
            shortcut.clone(),
            shell.clone(),
            javascript.clone(),
        ));

        Self {
            shortcut,
            queue,
            queue_signal,
            _handle: handle,
        }
    }

    pub fn press_chord(
        &self,
        runtime: Arc<ChordRuntime>,
        chord_payload: &ChordPayload,
    ) -> Result<()> {
        self.enqueue_action(QueuedChordAction::Press {
            runtime,
            chord_payload: chord_payload.clone(),
        });
        Ok(())
    }

    pub fn release_chord(&self, chord: &Chord) -> Result<()> {
        self.enqueue_action(QueuedChordAction::Release {
            chord: chord.clone(),
        });
        Ok(())
    }

    fn enqueue_action(&self, action: QueuedChordAction) {
        self.queue.lock().push_back(action);
        self.queue_signal.notify_one();
    }

    async fn process_queue(
        queue: Arc<Mutex<VecDeque<QueuedChordAction>>>,
        queue_signal: Arc<Notify>,
        shortcut: ChordShortcutRunner,
        shell: ChordShellRunner,
        javascript: ChordJavascriptRunner,
    ) {
        loop {
            let Some(action) = queue.lock().pop_front() else {
                queue_signal.notified().await;
                continue;
            };

            if let Err(error) = Self::run_action(action, &shortcut, &shell, &javascript).await {
                log::error!("failed to process queued chord action: {error}");
            }
        }
    }

    async fn run_action(
        action: QueuedChordAction,
        shortcut: &ChordShortcutRunner,
        shell: &ChordShellRunner,
        javascript: &ChordJavascriptRunner,
    ) -> Result<()> {
        match action {
            QueuedChordAction::Press {
                runtime,
                chord_payload,
            } => {
                if let Some(shortcut_binding) = chord_payload.chord.shortcut.clone() {
                    shortcut.press_shortcut(shortcut_binding, chord_payload.num_times)?;
                }

                if let Some(shell_command) = chord_payload.chord.shell.clone() {
                    shell.run_shell_command(shell_command).await;
                }

                if let Some(js) = chord_payload.chord.js.clone() {
                    log::debug!("Running JavaScript: {:?}", js);
                    javascript
                        .execute_chord_javascript(runtime.path.clone(), js, chord_payload.num_times)
                        .await?;
                }
            }
            QueuedChordAction::Release { chord } => {
                if let Some(shortcut_binding) = chord.shortcut.clone() {
                    shortcut.release_shortcut(shortcut_binding)?;
                }
            }
        }

        Ok(())
    }
}
