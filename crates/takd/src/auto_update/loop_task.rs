//! The periodic auto-update loop and its single tick.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tak_update::install_target::{Updatability, resolve_running_binary, updatability};
use tak_update::plan::{UpdateAction, UpdateOutcome};
use tak_update::release_client::{DEFAULT_REPO, RELEASE_PUBLIC_KEY};
use tak_update::runner::{SelfUpdateRequest, self_update};

use crate::agent::{AutoUpdateConfig, read_config};
use crate::daemon::remote::SubmitAttemptStore;

use super::drain::{DrainOutcome, wait_until_idle};
use super::state;

/// Spawn the background auto-update loop. No-op when disabled, when the kill
/// switch is set, or when the binary is not in a self-updatable location.
///
/// ```no_run
/// # // Reason: spawns a background tokio task, reads config from disk, and needs constructed daemon state.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn spawn_update_loop(
    config_root: PathBuf,
    state_root: PathBuf,
    store: SubmitAttemptStore,
) {
    tokio::spawn(async move {
        let Ok(config) = read_config(&config_root) else {
            return;
        };
        if !config.auto_update.loop_enabled(&config.transport) {
            tracing::info!("takd auto-update is disabled for this node");
            return;
        }
        if kill_switch_set() {
            tracing::info!("takd auto-update disabled via TAKD_NO_AUTO_UPDATE");
            return;
        }
        if !binary_is_self_updatable() {
            return;
        }
        tracing::info!("takd auto-update loop started");
        loop {
            tokio::time::sleep(jittered_interval(&config.auto_update)).await;
            if let Err(err) = run_tick(&config_root, &state_root, &store).await {
                tracing::warn!("takd auto-update tick failed (non-fatal): {err:#}");
            }
        }
    });
}

fn binary_is_self_updatable() -> bool {
    match resolve_running_binary() {
        Ok(exe) if updatability(&exe) == Updatability::Updatable => true,
        Ok(_) => {
            tracing::info!("takd auto-update: binary is not in a self-updatable location");
            false
        }
        Err(err) => {
            tracing::warn!("takd auto-update: cannot resolve own path: {err:#}");
            false
        }
    }
}

fn kill_switch_set() -> bool {
    matches!(
        std::env::var("TAKD_NO_AUTO_UPDATE").ok().as_deref(),
        Some("1" | "true" | "TRUE"),
    )
}

async fn run_tick(config_root: &Path, state_root: &Path, store: &SubmitAttemptStore) -> Result<()> {
    let config = read_config(config_root)?;
    let auto = config.auto_update;
    if !auto.loop_enabled(&config.transport) {
        return Ok(());
    }
    if !matches!(
        run_self_update(&auto, true).await?.action,
        UpdateAction::Available
    ) {
        return Ok(());
    }
    if !auto.auto_apply {
        tracing::info!("takd auto-update: a newer version is available (auto_apply disabled)");
        return Ok(());
    }
    let deadline = Duration::from_secs(auto.drain_timeout_secs);
    if wait_until_idle(store, deadline).await == DrainOutcome::DeadlineExceeded {
        tracing::warn!("takd auto-update: drain deadline exceeded; applying anyway");
    }
    let applied = run_self_update(&auto, false).await?;
    if let UpdateAction::Installed(report) = applied.action {
        if let Err(err) = state::record_pending(state_root, &applied.tag, &report.backups) {
            // The binaries are already swapped on disk but we could not record the
            // state needed for boot-time rollback. Revert the swap so disk matches
            // the still-running (old) binary, and keep running instead of exiting.
            tracing::error!(
                "takd auto-update: failed to record update state ({err:#}); reverting swap",
            );
            state::restore_backups(&report.backups);
            return Ok(());
        }
        tracing::info!(
            "takd auto-update: installed {}; exiting for supervisor restart",
            applied.tag,
        );
        std::process::exit(0);
    }
    Ok(())
}

async fn run_self_update(auto: &AutoUpdateConfig, check_only: bool) -> Result<UpdateOutcome> {
    let repo = auto
        .repo
        .clone()
        .unwrap_or_else(|| DEFAULT_REPO.to_string());
    let pinned = auto.pinned_version.clone();
    let include_sibling = auto.include_sibling_tak;
    let allow_downgrade = auto.allow_downgrade;
    tokio::task::spawn_blocking(move || {
        self_update(&SelfUpdateRequest {
            primary_name: "takd",
            current_version: env!("TAKD_VERSION"),
            repo: &repo,
            check_only,
            force: false,
            allow_downgrade,
            requested_tag: pinned.as_deref(),
            include_sibling,
            public_key: RELEASE_PUBLIC_KEY,
        })
    })
    .await?
}

fn jittered_interval(auto: &AutoUpdateConfig) -> Duration {
    let base = Duration::from_secs(auto.check_interval_hours.max(1) * 3600);
    if auto.jitter_hours == 0 {
        return base;
    }
    let span = auto.jitter_hours * 3600;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::from(elapsed.subsec_nanos()))
        .unwrap_or(0)
        ^ u64::from(std::process::id());
    base + Duration::from_secs(seed % span)
}
