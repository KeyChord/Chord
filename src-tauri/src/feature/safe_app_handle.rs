use crate::feature::app_handle_ext::AppManaged;
use crate::observables::{
    AppPermissionsObservable, AppSettingsObservable, ChorderObservable, GitReposObservable,
    Observable, get_all_observable_states,
};
use anyhow::Result;
use delegate::delegate;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri::{Emitter, Manager, Wry};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{Dialog, DialogExt};
use tauri_plugin_store::{Store, StoreExt};

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

// If we're converting an AppHandle, it means we have access to an AppHandle which already means
// it's safe
impl From<AppHandle> for SafeAppHandle {
    fn from(handle: AppHandle) -> Self {
        Self {
            inner: Arc::new(Inner {
                handle: handle.clone(),
                state: Mutex::new(State {
                    is_safe: true,
                    callbacks: Vec::new(),
                }),
            }),
        }
    }
}

impl SafeAppHandle {
    pub fn new(handle: AppHandle) -> Result<Self> {
        let safe_handle = Self {
            inner: Arc::new(Inner {
                handle: handle.clone(),
                state: Mutex::new(State {
                    is_safe: false,
                    callbacks: Vec::new(),
                }),
            }),
        };

        Ok(safe_handle)
    }

    pub fn on_safe<F>(&self, callback: F)
    where
        F: FnOnce(&AppHandle) + Send + 'static,
    {
        let mut state = self.inner.state.lock().expect("state mutex poisoned");

        if state.is_safe {
            drop(state);
            callback(&self.inner.handle);
        } else {
            state.callbacks.push(Box::new(callback));
        }
    }

    pub fn manage(&self, managed: AppManaged) {
        managed.register(&self.inner.handle);

        let callbacks = {
            let mut state = self.inner.state.lock().expect("state mutex poisoned");

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

    /// This is intentionally read-only; only the observable "owner" can write to the observable
    pub fn observable_state<T: Observable>(&self) -> Result<Arc<T::State>> {
        let observable = self.inner.handle.state::<Arc<T>>();
        Ok(observable.get_state()?)
    }

    pub fn handle(&self) -> &AppHandle {
        &self.inner.handle
    }
}

// Methods that can be called without `.state`
impl SafeAppHandle {
    pub fn new_webview_window_builder<L: Into<String>>(
        &self,
        label: L,
        url: WebviewUrl,
    ) -> Result<WebviewWindowBuilder<'_, Wry, AppHandle>> {
        let label = label.into();
        let observables = get_all_observable_states(self.clone())?;
        Ok(
            WebviewWindowBuilder::new(self.handle(), label, url).initialization_script(format!(
                r#"
          window.__INITIAL_STATES__ = {}
        "#,
                &serde_json::to_string(&observables)?
            )),
        )
    }

    delegate! {
        to self.handle() {
            pub fn autolaunch(&self) -> tauri::State<'_, tauri_plugin_autostart::AutoLaunchManager>;
            pub fn store(&self, path: impl AsRef<std::path::Path>) -> tauri_plugin_store::Result<Arc<Store<Wry>>>;
            pub fn path(&self) -> &tauri::path::PathResolver<Wry>;
            pub fn dialog(&self) -> &Dialog<Wry>;
            pub fn run_on_main_thread<F: FnOnce() + Send + 'static>(&self, f: F) -> tauri::Result<()>;
            pub fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> tauri::Result<()>;
            pub fn get_webview_window(&self, label: &str) -> Option<tauri::webview::WebviewWindow<Wry>>;
        }
    }
}
