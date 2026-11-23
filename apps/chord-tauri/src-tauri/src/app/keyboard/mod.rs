#![allow(unused)]
mod app;
pub use app::*;
#[cfg_attr(not(target_os = "macos"), path = "portable.rs")]
mod keyboard;
pub use keyboard::*;

#[cfg(target_os = "macos")]
pub(crate) mod caps_guardian;
#[cfg(target_os = "macos")]
mod caps_hotkey;
#[cfg(target_os = "macos")]
mod caps_lock;
mod caps_lock_guard;
#[cfg(target_os = "macos")]
mod caps_remap;
#[cfg(target_os = "macos")]
mod caps_space_gap;
pub mod health;
#[cfg(target_os = "macos")]
mod key_catcher;
#[cfg(target_os = "macos")]
mod secure_input;
#[cfg(target_os = "macos")]
mod tap;
#[cfg(target_os = "macos")]
mod thread_priority;
#[cfg(target_os = "macos")]
pub mod voice_over;
#[cfg(not(target_os = "macos"))]
pub mod voice_over {
    pub fn uses_caps_lock_modifier() -> bool {
        false
    }
}
