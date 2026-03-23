use tauri::{AppHandle, Runtime, Wry};
use tauri_plugin_store::StoreExt;
use anyhow::Result;
use global_hotkey::GlobalHotkeyStore;
use repos::GitReposStore;

mod global_hotkey;
mod repos;

pub use global_hotkey::*;
pub use repos::*;
use crate::feature::SafeAppHandle;

pub trait AppHandleStoreExt<R: Runtime> {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore<R>>;
    fn git_repos_store(&self) -> Result<GitReposStore<R>>;
}

impl<R: Runtime> AppHandleStoreExt<R> for AppHandle<R> {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore<R>> {
       Ok(self.store("global-hotkeys.json").map(GlobalHotkeyStore::new)?)
    }

    fn git_repos_store(&self) -> Result<GitReposStore<R>> {
        Ok(self.store("repos.json").map(GitReposStore::new)?)
    }
}

impl AppHandleStoreExt<Wry> for SafeAppHandle {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore<Wry>> {
        Ok(self.store("global-hotkeys.json").map(GlobalHotkeyStore::new)?)
    }

    fn git_repos_store(&self) -> Result<GitReposStore<Wry>> {
        Ok(self.store("repos.json").map(GitReposStore::new)?)
    }
}
