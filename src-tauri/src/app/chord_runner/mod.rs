use self::javascript::ChordJavascriptRunner;
use self::runtime::ChordPayload;
use self::runtime::ChordRuntime;
use self::shell::ChordShellRunner;
use self::shortcut::ChordShortcutRunner;
use crate::app::SafeAppHandle;
use crate::app::chord_package::Chord;
use anyhow::Result;
use std::sync::Arc;

pub mod javascript;
pub mod registry;
pub mod runtime;
pub mod shell;
pub mod shortcut;

// TODO: registry should be part of ChordRunner
pub struct ChordRunner {
    pub shortcut: ChordShortcutRunner,
    shell: ChordShellRunner,
    javascript: ChordJavascriptRunner,

    handle: SafeAppHandle
}

impl ChordRunner {
    pub fn new(handle: SafeAppHandle) -> Self {
        let shortcut = ChordShortcutRunner::new(handle.clone());
        let shell = ChordShellRunner::new(handle.clone());
        let javascript = ChordJavascriptRunner::new(handle.clone());

        Self {
            shortcut,
            shell,
            javascript,
            handle,
        }
    }

    pub fn press_chord(
        &self,
        runtime: Arc<ChordRuntime>,
        chord_payload: &ChordPayload,
    ) -> Result<()> {
        if let Some(shortcut) = chord_payload.chord.shortcut.clone() {
            self.shortcut
                .press_shortcut(shortcut, chord_payload.num_times)?;
        }

        if let Some(shell) = chord_payload.chord.shell.clone() {
            self.shell.run_shell_command(shell);
        }

        if let Some(js) = chord_payload.chord.js.clone() {
            log::debug!("Running JavaScript: {:?}", js);
            let javascript = self.javascript.clone();
            let chord_payload = chord_payload.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = javascript
                    .execute_chord_javascript(runtime.path.clone(), js, chord_payload.num_times)
                    .await
                {
                    log::error!("failed to execute js: {}", e);
                };
            });
        }

        Ok(())
    }

    pub fn release_chord(&self, chord: &Chord) -> Result<()> {
        if let Some(shortcut) = chord.shortcut.clone() {
            self.shortcut.release_shortcut(shortcut)?;
        }

        Ok(())
    }
}
