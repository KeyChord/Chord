use tauri::{Emitter, Manager, Wry};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use delegate::delegate;
use serde::Serialize;
use tauri_plugin_dialog::{Dialog, DialogExt};
use tauri_plugin_store::{Store, StoreExt};
use crate::feature::app_handle_ext::AppManaged;
use crate::observables::get_all_observable_defaults;

type OnSafeCallback = Box<dyn FnOnce(&AppHandle) + Send + 'static>;

/// A safe handle is a wrapper around AppHandle that is guaranteed to not panic (i.e. it only exposes
/// "safe" methods). In contrast, calling `.state` on AppHandle will panic if the state hasn't been
/// managed yet.
#[derive(Clone)]
pub struct SafeAppHandle {
    inner: Arc<Inner>,
}

struct Inner {
    handle: AppHandle,
    state: Mutex<State>,
}

struct State {
    is_safe: bool,
    callbacks: Vec<OnSafeCallback>,
}

impl SafeAppHandle {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            inner: Arc::new(Inner {
                handle,
                state: Mutex::new(State {
                    is_safe: false,
                    callbacks: Vec::new(),
                }),
            }),
        }
    }

    pub fn on_safe<F>(&self, callback: F)
    where
        F: FnOnce(&AppHandle) + Send + 'static,
    {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("state mutex poisoned");

        if state.is_safe {
            drop(state);
            callback(&self.inner.handle);
        } else {
            state.callbacks.push(Box::new(callback));
        }
    }

    pub fn mark_safe(&self, managed: AppManaged) {
        managed.register(&self.inner.handle);

        let callbacks = {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("state mutex poisoned");

            if state.is_safe {
                return;
            }

            state.is_safe = true;
            std::mem::take(&mut state.callbacks)
        };

        for callback in callbacks {
            callback(&self.inner.handle);
        }
    }

    pub fn handle(&self) -> &AppHandle {
        &self.inner.handle
    }
}

// Methods that can be called without `.state`
impl SafeAppHandle {
    pub fn is_autolaunch_enabled(&self) -> Result<bool> {
        Ok(self.handle().autolaunch().is_enabled()?)
    }

    pub fn new_webview_window_builder<L: Into<String>>(
        &self,
        label: L,
        url: WebviewUrl,
    ) -> Result<WebviewWindowBuilder<'_, Wry, AppHandle>> {
        let label = label.into();
        let defaults = get_all_observable_defaults()?;
        Ok(WebviewWindowBuilder::new(self.handle(), label, url).initialization_script(format!(r#"
          window.__INITIAL_STATES__ = {}
        "#, &serde_json::to_string(&defaults)?)))
    }

    delegate! {
        to self.handle() {
            pub fn store(&self, path: impl AsRef<std::path::Path>) -> tauri_plugin_store::Result<Arc<Store<Wry>>>;
            pub fn path(&self) -> &tauri::path::PathResolver<Wry>;
            pub fn dialog(&self) -> &Dialog<Wry>;
            pub fn run_on_main_thread<F: FnOnce() + Send + 'static>(&self, f: F) -> tauri::Result<()>;
            pub fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> tauri::Result<()>;
        }
    }
}
