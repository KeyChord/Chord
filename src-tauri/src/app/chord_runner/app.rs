use tauri::AppHandle;
use crate::app::chord_runner::{ChordActionTaskRunner, HandlerChordActionTaskRunner, ShellChordActionTaskRunner, ShortcutChordActionTaskRunner};
use crate::app::state::AppSingleton;

impl<T> AppSingleton<T> for ChordActionTaskRunner {
    fn new(handle: AppHandle) -> Self {
        Self {
            handler: HandlerChordActionTaskRunner::new(handle.clone()),
            shell: ShellChordActionTaskRunner::new(handle.clone()),
            shortcut: ShortcutChordActionTaskRunner::new(handle.clone()),
        }
    }
}