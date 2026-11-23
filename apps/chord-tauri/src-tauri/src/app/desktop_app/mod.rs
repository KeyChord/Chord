#![allow(unused)]
mod app;
pub use app::*;
#[cfg_attr(not(target_os = "macos"), path = "portable_identity.rs")]
mod app_identity;
pub use app_identity::*;
mod desktop_app;
pub use desktop_app::*;

mod installed_apps;
#[cfg(not(target_os = "macos"))]
pub(crate) use crate::desktop_platform::installed_apps;
#[cfg(target_os = "macos")]
pub use installed_apps::*;

mod desktop_app_manager;
pub use desktop_app_manager::*;
