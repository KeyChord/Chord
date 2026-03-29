use serde::Serialize;
use typeshare::typeshare;
use crate::models::shortcut_simulation::SimulatedShortcut;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub enum ChordAction {
    Shortcut(ChordShortcutAction),
    Shell(ChordShellAction),
    Javascript(ChordJavascriptAction)
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordShortcutAction {
    pub simulated_shortcut: SimulatedShortcut
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordShellAction {
    pub command: String
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct ChordJavascriptAction {
    pub export_name: String,
    pub args: Vec<toml::Value>
}
