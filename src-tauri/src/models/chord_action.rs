use serde::Serialize;
use typeshare::typeshare;
use crate::models::shortcut_simulation::SimulatedShortcut;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum ChordAction {
    Shortcut(ShortcutChordAction),
    Shell(ShellChordAction),
    Javascript(JavascriptChordAction)
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ShortcutChordAction {
    pub simulated_shortcut: SimulatedShortcut
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ShellChordAction {
    pub command: String
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct JavascriptChordAction {
    pub module_specifier: String,
    pub args: Vec<toml::Value>
}
