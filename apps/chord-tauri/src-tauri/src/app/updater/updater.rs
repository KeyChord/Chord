//! Keeps Chord up to date. Each release publishes `latest.json` beside its signed app archives.
//! Chord reads it at launch and once a day and offers anything newer in Settings and the tray
//! menu. Downloading, signature checks and replacing the app are `tauri-plugin-updater`'s job.

use crate::app::{AppHandleExt, AppSingleton};
use crate::state::{AppUpdateObservable, AppUpdateState, Observable};
use crate::tauri_app::tray;
use anyhow::Result;
use nject::{injectable, provider};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Url};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Points Chord at another `latest.json`, for testing an update. Whatever it lists must still be
/// signed with the key whose public half is in tauri.conf.json, so it can't install anything else.
const ENDPOINT_ENV: &str = "CHORD_UPDATE_ENDPOINT";

#[provider]
pub struct AppUpdaterProvider {
    #[provide(AppUpdateObservable, |v| v.provide())]
    pub app_update_observable: AppUpdateObservable,

    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

#[injectable]
pub struct AppUpdater {
    observable: AppUpdateObservable,
    handle: AppHandle,
    /// The release on offer. Also guards moves between the update states.
    #[inject(Mutex::new(None))]
    found: Mutex<Option<Update>>,
}

impl AppSingleton for AppUpdater {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}

impl AppUpdater {
    pub fn state(&self) -> Result<AppUpdateState> {
        self.observable.get_state()
    }

    pub fn start(&self) {
        // A development build isn't from a release, so a release would always look newer.
        if cfg!(debug_assertions) {
            return;
        }
        let handle = self.handle.clone();
        tauri::async_runtime::spawn(async move {
            let updater = handle.app_state().app_updater();
            loop {
                if let Err(error) = updater.check().await {
                    log::warn!("Couldn't check for a Chord update: {error:#}");
                }
                tokio::time::sleep(CHECK_INTERVAL).await;
            }
        });
    }

    async fn check(&self) -> Result<()> {
        if self.state()?.is_applying() {
            return Ok(());
        }
        let mut builder = self.handle.updater_builder();
        if let Some(endpoint) = std::env::var(ENDPOINT_ENV)
            .ok()
            .filter(|value| !value.is_empty())
        {
            log::info!("Checking for updates at {endpoint}");
            builder = builder.endpoints(vec![Url::parse(&endpoint)?])?;
        }
        let update = builder.build()?.check().await?;
        self.record_check(update)
    }

    fn record_check(&self, update: Option<Update>) -> Result<()> {
        {
            let mut found = self.found.lock().expect("poisoned");
            // Someone may have started installing while the check was running.
            if self.state()?.is_applying() {
                return Ok(());
            }
            let state = match &update {
                Some(update) => AppUpdateState::Available {
                    version: update.version.clone(),
                },
                None => AppUpdateState::Idle,
            };
            *found = update;
            if self.state()? == state {
                return Ok(());
            }
            self.observable.set_state(|_| state)?;
        }
        self.refresh_tray()
    }

    /// Downloads and installs the release on offer. Chord keeps running the old build until it
    /// restarts. Returns once that's done, and does nothing if there's no update to start.
    pub async fn install(&self) -> Result<()> {
        let Some(update) = self.begin_install()? else {
            return Ok(());
        };
        let version = update.version.clone();
        log::info!("Installing Chord {version} over {}", update.current_version);
        match self.download_and_install(update).await {
            Ok(()) => {
                log::info!("Installed Chord {version}");
                self.publish(AppUpdateState::Ready { version })
            }
            Err(error) => {
                // The offer stands, so the card and the menu item can try again.
                self.publish(AppUpdateState::Available { version })?;
                Err(error)
            }
        }
    }

    fn begin_install(&self) -> Result<Option<Update>> {
        let update = {
            let found = self.found.lock().expect("poisoned");
            let Some(update) = found.clone() else {
                return Ok(None);
            };
            // Two clicks can't start two downloads.
            if !matches!(self.state()?, AppUpdateState::Available { .. }) {
                return Ok(None);
            }
            let version = update.version.clone();
            self.observable.set_state(|_| AppUpdateState::Downloading {
                version,
                percent: None,
            })?;
            update
        };
        self.refresh_tray()?;
        Ok(Some(update))
    }

    async fn download_and_install(&self, update: Update) -> Result<()> {
        let version = update.version.clone();
        let mut progress = DownloadProgress::default();
        let bytes = update
            .download(
                |chunk, total| {
                    let Some(percent) = progress.advance(chunk, total) else {
                        return;
                    };
                    let version = version.clone();
                    let state = AppUpdateState::Downloading {
                        version,
                        percent: Some(percent),
                    };
                    if let Err(error) = self.observable.set_state(|_| state) {
                        log::warn!("Couldn't report update progress: {error:#}");
                    }
                },
                || {},
            )
            .await?;
        self.publish(AppUpdateState::Installing { version })?;
        // Replacing the app can wait on someone typing an administrator password.
        tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await??;
        Ok(())
    }

    /// Starts the new build. Only an installed update has anything to restart into.
    pub fn restart(&self) -> Result<()> {
        anyhow::ensure!(
            matches!(self.state()?, AppUpdateState::Ready { .. }),
            "There's no installed update to restart into"
        );
        self.handle.restart()
    }

    /// The tray has no toast to report through, so a failure from there gets a dialog.
    pub fn on_menu_item_clicked(&self) {
        match self.state() {
            Ok(AppUpdateState::Available { .. }) => {
                let handle = self.handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = handle.app_state().app_updater().install().await {
                        log::error!("Couldn't install the Chord update: {error:#}");
                        handle
                            .dialog()
                            .message(format!("Chord couldn't install the update: {error:#}"))
                            .title("Update failed")
                            .kind(MessageDialogKind::Error)
                            .show(|_| {});
                    }
                });
            }
            Ok(AppUpdateState::Ready { .. }) => self.handle.restart(),
            Ok(_) => {}
            Err(error) => log::error!("Couldn't read the update state: {error:#}"),
        }
    }

    fn publish(&self, state: AppUpdateState) -> Result<()> {
        self.observable.set_state(|_| state)?;
        self.refresh_tray()
    }

    fn refresh_tray(&self) -> Result<()> {
        tray::refresh_menu(&self.handle, &self.state()?)?;
        Ok(())
    }
}

/// Turns the updater's chunk sizes into a whole percentage. It reports only changes, because a
/// download arrives in thousands of chunks and each report reaches every open window.
#[derive(Default)]
struct DownloadProgress {
    downloaded: u64,
    percent: Option<u32>,
}

impl DownloadProgress {
    fn advance(&mut self, chunk: usize, total: Option<u64>) -> Option<u32> {
        self.downloaded += chunk as u64;
        let total = total.filter(|total| *total > 0)?;
        let percent = (self.downloaded.saturating_mul(100) / total).min(100) as u32;
        if self.percent == Some(percent) {
            return None;
        }
        self.percent = Some(percent);
        Some(percent)
    }
}

#[cfg(test)]
mod tests {
    use super::DownloadProgress;

    #[test]
    fn reports_each_whole_percent_once() {
        let mut progress = DownloadProgress::default();
        assert_eq!(progress.advance(10, Some(200)), Some(5));
        assert_eq!(progress.advance(1, Some(200)), None);
        assert_eq!(progress.advance(1, Some(200)), Some(6));
    }

    #[test]
    fn reports_nothing_without_a_size() {
        let mut progress = DownloadProgress::default();
        assert_eq!(progress.advance(10, None), None);
        assert_eq!(progress.advance(10, Some(0)), None);
    }

    #[test]
    fn stops_at_one_hundred_when_the_size_was_understated() {
        let mut progress = DownloadProgress::default();
        assert_eq!(progress.advance(300, Some(200)), Some(100));
        assert_eq!(progress.advance(10, Some(200)), None);
    }
}
