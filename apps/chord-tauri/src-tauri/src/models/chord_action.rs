use super::{ChordsFileHandlerKind, SimulatedShortcut};
use serde::Serialize;
use typeshare::typeshare;

/// The action that a chord can define.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "content")]
pub enum ChordAction {
    Shortcut(ShortcutChordAction),
    Shell(ShellChordAction),
    Emit(EmitChordAction),
}

/// The action that a chord task is meant to execute.
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "content")]
pub enum ChordTaskAction {
    Shortcut(ShortcutChordAction),
    Shell(ShellChordAction),
    Handler(HandlerChordAction),
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutChordAction {
    pub simulated_shortcut: SimulatedShortcut,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellChordAction {
    pub command: String,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmitChordAction {
    pub event_key: String,
    #[typeshare(typescript(type = "any"))]
    pub args: Vec<toml::Value>,
}

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandlerChordAction {
    /// JS: key in the QuickJS `__RUST_HANDLER_REGISTRY`. Native: registration id in the
    /// native host's active generation.
    pub handler_id: String,
    pub kind: ChordsFileHandlerKind,
    #[typeshare(typescript(type = "any[]"))]
    pub event_args: Vec<toml::Value>,
}
