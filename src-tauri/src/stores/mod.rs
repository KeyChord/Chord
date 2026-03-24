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

pub trait AppHandleStoreExt {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore>;
    fn git_repos_store(&self) -> Result<GitReposStore>;
}

impl AppHandleStoreExt for AppHandle<Wry> {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore> {
        Ok(GlobalHotkeyStore::new(self.clone().into())?)
    }

    fn git_repos_store(&self) -> Result<GitReposStore> {
        Ok(GitReposStore::new(self.clone().into())?)
    }
}

impl AppHandleStoreExt for SafeAppHandle {
    fn global_hotkeys_store(&self) -> Result<GlobalHotkeyStore> {
        GlobalHotkeyStore::new(self.clone())
    }

    fn git_repos_store(&self) -> Result<GitReposStore> {
        GitReposStore::new(self.clone())
    }
}
