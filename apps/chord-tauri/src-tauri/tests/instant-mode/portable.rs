//! Production models and keyboard entry point, with action execution and Tauri services simulated.
#![allow(dead_code, unused_imports)]
#[macro_export]
macro_rules! define_observable {
    ($($tokens:tt)*) => {};
}
#[path = "../../src/models/chord_command.rs"]
mod chord_command;
#[path = "../../src/models/chord_escape.rs"]
mod chord_escape;
#[path = "../../src/models/chord_keyboard_event.rs"]
mod chord_keyboard_event;
#[path = "../../src/models/chord_layout.rs"]
mod chord_layout;
#[path = "../../src/models/chord_macros.rs"]
mod chord_macros;
#[path = "../../src/models/key.rs"]
mod key;
mod models {
    pub use crate::{
        chord_command::*, chord_escape::*, chord_keyboard_event::*, chord_layout::*,
        chord_macros::*, key::*,
    };
    include!(concat!(env!("OUT_DIR"), "/models.rs"));
}
pub use models::*;
#[path = "../../src/state/observables/chord_input.rs"]
mod chord_input;
mod state {
    pub use crate::chord_input::*;
}
#[cfg(test)]
mod entry;
#[path = "../../src/app/chord_mode_manager/instant_mode.rs"]
mod instant_mode;
