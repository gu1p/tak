//! Persistent update state for boot-time health confirmation and crash-loop
//! rollback.
//!
//! After the loop swaps the binary it records the new tag and the `.bak` paths,
//! then exits for the supervisor to restart into the new binary. On the next
//! start the daemon [`reconcile_on_start`]s: if it survives long enough to
//! [`commit`], the backups are deleted; if it crash-loops past
//! [`MAX_BOOT_ATTEMPTS`], the backups are restored and the daemon exits so the
//! supervisor brings back the previous, known-good binary. State writes are
//! atomic (temp + fsync + rename) so a crash mid-write can't corrupt them.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "update-state.toml";

/// Restarts allowed before a freshly-installed binary is rolled back.
pub(crate) const MAX_BOOT_ATTEMPTS: u32 = 3;

/// A pending update awaiting boot-time health confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PendingUpdate {
    /// The tag that was just installed.
    pub tag: String,
    /// `.bak` files (one per swapped binary) to delete on commit / restore on rollback.
    pub backups: Vec<PathBuf>,
    /// Restarts observed since the swap.
    #[serde(default)]
    pub boot_attempts: u32,
}

/// What [`reconcile_on_start`] decided.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BootDecision {
    /// No update was pending; normal start.
    None,
    /// An update is pending; proceed and confirm health shortly.
    Proceed,
    /// The new binary crash-looped; backups were restored, exit to restart.
    RolledBack,
}

fn state_path(state_root: &Path) -> PathBuf {
    state_root.join(STATE_FILE)
}

fn write_state(state_root: &Path, pending: &PendingUpdate) -> Result<()> {
    let path = state_path(state_root);
    let tmp = path.with_extension("toml.tmp");
    let encoded = toml::to_string(pending).context("encode update state")?;
    let mut file = File::create(&tmp).context("create update state temp")?;
    file.write_all(encoded.as_bytes())
        .and_then(|()| file.sync_all())
        .context("write update state temp")?;
    fs::rename(&tmp, &path).context("install update state")?;
    Ok(())
}

/// Record a freshly-applied update so the next boot can confirm or roll it back.
pub(crate) fn record_pending(state_root: &Path, tag: &str, backups: &[PathBuf]) -> Result<()> {
    write_state(
        state_root,
        &PendingUpdate {
            tag: tag.to_string(),
            backups: backups.to_vec(),
            boot_attempts: 0,
        },
    )
}

/// Read the pending update, if any. Logs and ignores a malformed state file.
pub(crate) fn read_pending(state_root: &Path) -> Option<PendingUpdate> {
    let raw = fs::read_to_string(state_path(state_root)).ok()?;
    match toml::from_str(&raw) {
        Ok(pending) => Some(pending),
        Err(err) => {
            tracing::warn!("auto-update: ignoring malformed update-state: {err}");
            None
        }
    }
}

/// Mark the current binary healthy: delete the backups and clear the state.
pub(crate) fn commit(state_root: &Path) {
    if let Some(pending) = read_pending(state_root) {
        for backup in &pending.backups {
            let _ = fs::remove_file(backup);
        }
    }
    let _ = fs::remove_file(state_path(state_root));
}

/// Restore each `<target>.bak` over `<target>` (used to revert a failed swap).
pub(crate) fn restore_backups(backups: &[PathBuf]) {
    for backup in backups {
        restore_backup(backup);
    }
}

/// Increment the boot counter and decide whether to keep running or roll back.
pub(crate) fn reconcile_on_start(state_root: &Path) -> BootDecision {
    let Some(mut pending) = read_pending(state_root) else {
        return BootDecision::None;
    };
    pending.boot_attempts += 1;
    if pending.boot_attempts > MAX_BOOT_ATTEMPTS {
        restore_backups(&pending.backups);
        let _ = fs::remove_file(state_path(state_root));
        return BootDecision::RolledBack;
    }
    if let Err(err) = write_state(state_root, &pending) {
        tracing::warn!("auto-update: failed to persist boot attempt: {err:#}");
    }
    BootDecision::Proceed
}

fn restore_backup(backup: &Path) {
    // `<target>.bak` -> `<target>` at the path level (no UTF-8 assumption).
    let target = backup.with_extension("");
    if let Err(err) = fs::rename(backup, &target) {
        tracing::error!(
            "auto-update rollback: failed to restore {}: {err:#}",
            target.display(),
        );
    }
}

#[path = "state_tests.rs"]
mod state_tests;
