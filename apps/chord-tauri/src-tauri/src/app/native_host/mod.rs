#![allow(unused)]
//! Supervision of the isolated `chord-native-host` process that executes native handlers.
//!
//! Chord owns exactly four things for native handlers: artifact discovery/validation
//! (`ChordNativePackage`), the host lifecycle and crash supervision (`NativeHostSupervisor`),
//! the invocation transport (`chord_native_protocol::client`), and the tiny C ABI the host
//! resolves. Everything else is the package author's native code.

mod app;
pub use app::*;
mod errors;
pub use errors::*;
pub mod materialize;
mod supervisor;
pub use supervisor::*;
