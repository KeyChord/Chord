use crate::feature::app_handle_ext::AppManaged;
use crate::observables::{Observable, get_all_observable_states};
use anyhow::Result;
use delegate::delegate;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{Dialog, DialogExt};
use tauri_plugin_store::{Store, StoreExt};

type OnSafeCallback = Box<dyn FnOnce(AppHandle) + Send + 'static>;
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait OnSafeReturn: Send + 'static {
    fn dispatch(self);
}

impl OnSafeReturn for () {
    fn dispatch(self) {}
}

impl<T, E> OnSafeReturn for Result<T, E>
where
    T: OnSafeReturn,
    E: std::fmt::Display + Send + 'static,
{
    fn dispatch(self) {
        match self {
            Ok(value) => value.dispatch(),
            Err(err) => {
                log::error!("on_safe callback failed: {err}");
            }
        }
    }
}

pub struct AsyncOnSafe<R> {
    future: BoxFuture<R>,
}

impl<R> AsyncOnSafe<R>
where
    R: OnSafeReturn,
{
    pub fn new<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = R> + Send + 'static,
    {
        Self {
            future: Box::pin(future),
        }
    }
}

impl<R> OnSafeReturn for AsyncOnSafe<R>
where
    R: OnSafeReturn,
{
    fn dispatch(self) {
        tauri::async_runtime::spawn(async move {
            self.future.await.dispatch();
        });
    }
}

/// A safe handle is a wrapper around AppHandle that is guaranteed to not panic
/// for operations that require managed state to already exist.
///
/// In contrast, calling `.state()` on `AppHandle` will panic if that state has
/// not been managed yet.
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

// If we're converting from an AppHandle, we already have the fully initialized handle,
// so treat it as safe immediately.
impl From<AppHandle> for SafeAppHandle {
    fn from(handle: AppHandle) -> Self {
        Self {
            inner: Arc::new(Inner {
                handle,
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
        Ok(Self {
            inner: Arc::new(Inner {
                handle,
                state: Mutex::new(State {
                    is_safe: false,
                    callbacks: Vec::new(),
                }),
            }),
        })
    }

    pub fn on_safe<F, R>(&self, callback: F)
    where
        F: FnOnce(AppHandle) -> R + Send + 'static,
        R: OnSafeReturn,
    {
        let wrapped: OnSafeCallback = Box::new(move |app| {
            callback(app).dispatch();
        });

        let maybe_run_now = {
            let mut state = self.inner.state.lock().expect("state mutex poisoned");

            if state.is_safe {
                Some(wrapped)
            } else {
                state.callbacks.push(wrapped);
                None
            }
        };

        if let Some(callback) = maybe_run_now {
            callback(self.inner.handle.clone());
        }
    }

    pub fn on_safe_async<F, Fut, R>(&self, callback: F)
    where
        F: FnOnce(AppHandle) -> Fut + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: OnSafeReturn,
    {
        self.on_safe(move |app| AsyncOnSafe::new(callback(app)));
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
            callback(self.inner.handle.clone());
        }
    }

    /// This is intentionally read-only; only the observable owner can write to it.
    pub fn observable_state<T: Observable>(&self) -> Result<Arc<T::State>> {
        let observable = self.inner.handle.state::<Arc<T>>();
        observable.get_state()
    }

    pub fn try_handle(&self) -> Option<&AppHandle> {
        let mut state = self.inner.state.lock().expect("state mutex poisoned");
        if state.is_safe {
            Some(&self.inner.handle)
        } else {
            None
        }
    }

    pub fn handle(&self) -> &AppHandle {
        &self.inner.handle
    }
}

// Methods that can be called without `.state()`
impl SafeAppHandle {
    pub fn new_webview_window_builder<L: Into<String>>(
        &self,
        label: L,
        url: WebviewUrl,
    ) -> Result<WebviewWindowBuilder<'_, Wry, AppHandle>> {
        Ok(WebviewWindowBuilder::new(self.handle(), label.into(), url))
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
