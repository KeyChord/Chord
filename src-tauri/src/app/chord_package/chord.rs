use crate::app::chord_runner::javascript::ChordJsInvocation;
use crate::app::chord_runner::shortcut::Shortcut;
use crate::input::Key;
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub index: u32,
    pub name: String,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    pub js: Option<ChordJsInvocation>,
}
