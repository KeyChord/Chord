//! Keeps Chord up to date. Each release publishes `latest.json` beside its signed app archives;
//! Chord reads it at launch and once a day, installs a newer build, and offers to restart into it.

use std::time::Duration;
use tauri::{AppHandle, Url};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Points Chord at another `latest.json`, for testing an update. Whatever it lists must still be
/// signed with the key whose public half is in tauri.conf.json, so it can't install anything else.
const ENDPOINT_ENV: &str = "CHORD_UPDATE_ENDPOINT";

pub fn start(handle: AppHandle) {
    // A development build isn't from a release, so a release would always look newer.
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            match install_update(&handle).await {
                Ok(Some(version)) => {
                    // The new build is on disk; there's nothing more to check until it runs.
                    offer_restart(&handle, &version);
                    return;
                }
                Ok(None) => {}
                Err(error) => log::warn!("Couldn't check for a Chord update: {error:#}"),
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

async fn install_update(handle: &AppHandle) -> anyhow::Result<Option<String>> {
    let mut builder = handle.updater_builder();
    if let Some(endpoint) = std::env::var(ENDPOINT_ENV).ok().filter(|value| !value.is_empty()) {
        log::info!("Checking for updates at {endpoint}");
        builder = builder.endpoints(vec![Url::parse(&endpoint)?])?;
    }
    let Some(update) = builder.build()?.check().await? else {
        return Ok(None);
    };
    log::info!("Installing Chord {} over {}", update.version, update.current_version);
    update.download_and_install(|_, _| {}, || {}).await?;
    log::info!("Installed Chord {}", update.version);
    Ok(Some(update.version))
}

fn offer_restart(handle: &AppHandle, version: &str) {
    let app = handle.clone();
    handle
        .dialog()
        .message(format!(
            "Chord {version} is installed. Restart Chord to start using it now, or it will be used the next time Chord opens."
        ))
        .title("Chord was updated")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Restart Chord".into(),
            "Later".into(),
        ))
        .show(move |restart| {
            if restart {
                app.restart();
            }
        });
}
