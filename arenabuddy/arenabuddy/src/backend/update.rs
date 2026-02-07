use std::sync::Arc;

use self_update::{self, cargo_crate_version, update::ReleaseUpdate};
use tokio::sync::Mutex;
use tracingx::{error, info};

#[derive(Clone, Debug)]
pub enum UpdateStatus {
    Checking,
    Available { version: String },
    UpToDate,
    Updating,
    RestartRequired,
    Error(String),
}

pub type SharedUpdateState = Arc<Mutex<UpdateStatus>>;

pub fn new_shared_update_state() -> SharedUpdateState {
    Arc::new(Mutex::new(UpdateStatus::Checking))
}

fn build_updater() -> Result<Box<dyn ReleaseUpdate>, self_update::errors::Error> {
    self_update::backends::github::Update::configure()
        .repo_owner("gazure")
        .repo_name("monorepo")
        .bin_name("arenabuddy")
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .show_download_progress(false)
        .build()
}

pub async fn check_for_update(state: SharedUpdateState) {
    let result = tokio::task::spawn_blocking(|| {
        let updater = build_updater()?;
        let latest = updater.get_latest_release()?;
        let current = cargo_crate_version!();
        info!("Current version: {current}, latest release: {}", latest.version);
        Ok::<_, self_update::errors::Error>(latest.version)
    })
    .await;

    let mut s = state.lock().await;
    match result {
        Ok(Ok(latest_version)) => {
            let current = cargo_crate_version!();
            if latest_version.trim_start_matches('v') > current {
                info!("Update available: {latest_version}");
                *s = UpdateStatus::Available {
                    version: latest_version,
                };
            } else {
                info!("Already up to date");
                *s = UpdateStatus::UpToDate;
            }
        }
        Ok(Err(e)) => {
            error!("Failed to check for updates: {e}");
            *s = UpdateStatus::Error(e.to_string());
        }
        Err(e) => {
            error!("Update check task panicked: {e}");
            *s = UpdateStatus::Error(e.to_string());
        }
    }
}

pub async fn apply_update(state: SharedUpdateState) {
    {
        *state.lock().await = UpdateStatus::Updating;
    }

    let result = tokio::task::spawn_blocking(|| {
        let updater = build_updater()?;
        let status = updater.update()?;
        Ok::<_, self_update::errors::Error>(status)
    })
    .await;

    let mut s = state.lock().await;
    match result {
        Ok(Ok(status)) => {
            if status.updated() {
                info!("Updated to {}", status.version());
                *s = UpdateStatus::RestartRequired;
            } else {
                info!("Already up to date: {}", status.version());
                *s = UpdateStatus::UpToDate;
            }
        }
        Ok(Err(e)) => {
            error!("Failed to apply update: {e}");
            *s = UpdateStatus::Error(e.to_string());
        }
        Err(e) => {
            error!("Update task panicked: {e}");
            *s = UpdateStatus::Error(e.to_string());
        }
    }
}
