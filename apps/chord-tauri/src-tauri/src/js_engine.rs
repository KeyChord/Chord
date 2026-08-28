//! Which JavaScript engine runs chord handlers: Bun (rbun) — the default, and
//! the engine that gives packages `bun:ffi` for native code — or QuickJS
//! (rquickjs + LLRT), the legacy engine, kept for compatibility and used
//! automatically by a build without the `bun` cargo feature.
//!
//! The choice is read once per process — from `CHORD_JS_ENGINE` if set,
//! otherwise from the `jsEngine` key of the app state store — the first time
//! the engine is needed, so switching in the settings UI takes effect after a
//! restart.

use crate::startup::APP_STATE_STORE_PATH;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsEngine {
    QuickJs,
    Bun,
}

impl Default for JsEngine {
    /// Bun whenever this build has it, QuickJS otherwise.
    fn default() -> Self {
        if bun_available() {
            JsEngine::Bun
        } else {
            JsEngine::QuickJs
        }
    }
}

impl JsEngine {
    pub fn parse(value: &str) -> Option<JsEngine> {
        match value.trim().to_ascii_lowercase().as_str() {
            "quickjs" | "rquickjs" | "llrt" => Some(JsEngine::QuickJs),
            "bun" | "rbun" => Some(JsEngine::Bun),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            JsEngine::QuickJs => "quickjs",
            JsEngine::Bun => "bun",
        }
    }
}

pub const STORE_KEY: &str = "jsEngine";
pub const ENV_VAR: &str = "CHORD_JS_ENGINE";

/// Whether this binary was built with Bun support (`--features bun`).
pub const fn bun_available() -> bool {
    cfg!(feature = "bun")
}

static SELECTED: OnceLock<JsEngine> = OnceLock::new();

/// The engine persisted in the app state store (not the one currently
/// running).
pub fn configured(handle: &AppHandle) -> JsEngine {
    if let Some(engine) = std::env::var(ENV_VAR).ok().and_then(|v| JsEngine::parse(&v)) {
        return engine;
    }
    handle
        .store(APP_STATE_STORE_PATH)
        .ok()
        .and_then(|store| store.get(STORE_KEY))
        .and_then(|value| value.as_str().and_then(JsEngine::parse))
        .unwrap_or_default()
}

/// The engine this process runs; decided on first use.
pub fn select(handle: &AppHandle) -> JsEngine {
    *SELECTED.get_or_init(|| {
        let engine = configured(handle);
        if engine == JsEngine::Bun && !bun_available() {
            log::warn!("JS engine `bun` requested but this build has no Bun support (build with `--features bun`); using QuickJS");
            return JsEngine::QuickJs;
        }
        log::info!("JS engine: {}", engine.as_str());
        engine
    })
}

/// The engine selected so far (the default until [`select`] has run).
pub fn current() -> JsEngine {
    SELECTED.get().copied().unwrap_or_default()
}

/// Engine for standalone (CLI) runs, where there is no app handle.
///
/// `requested` is the `--engine` flag when one was passed; otherwise the choice comes from
/// `CHORD_JS_ENGINE`, then the default. Like [`select`], the answer is decided once per process.
pub fn select_for_cli(requested: Option<JsEngine>) -> JsEngine {
    *SELECTED.get_or_init(|| {
        let engine = requested
            .or_else(|| std::env::var(ENV_VAR).ok().and_then(|v| JsEngine::parse(&v)))
            .unwrap_or_default();
        if engine == JsEngine::Bun && !bun_available() {
            eprintln!("warning: Bun engine requested but this build has no Bun support (build with `--features bun`); using QuickJS");
            return JsEngine::QuickJs;
        }
        engine
    })
}
