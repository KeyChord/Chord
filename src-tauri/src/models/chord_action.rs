use serde::Serialize;
use typeshare::typeshare;
use crate::models::shortcut_simulation::SimulatedShortcut;

/// The action that a chord can define.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum ChordAction {
    Shortcut(ShortcutChordAction),
    Shell(ShellChordAction),
    Emit(EmitChordAction),
}

/// The action that a chord task is meant to execute.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum ChordTaskAction {
    Shortcut(ShortcutChordAction),
    Shell(ShellChordAction),
    Handler(HandlerChordAction)
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
pub struct EmitChordAction {
    pub event_key: String,
    pub args: Vec<toml::Value>
}

/// Currently, we only support JavaScript handlers
#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct HandlerChordAction {
    pub file: String,
    pub handler_args: Vec<toml::Value>,
    pub event_args: Vec<toml::Value>
}
