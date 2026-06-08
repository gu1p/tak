//! Filesystem [`Installer`]: validate every candidate, then swap with rollback.
//!
//! Two passes give the safety guarantee: pass one executes each staged
//! candidate's `--version` (in a temp file, never the live path) and aborts the
//! whole update on the first mismatch — so a bad download never touches a live
//! binary. Pass two backs up and swaps each target; if any commit fails after
//! others succeeded, the committed ones are rolled back (pre-existing targets
//! restored from `.bak`, freshly-created targets removed).
//!
//! This is **process-failure** rollback, not crash-atomicity: a hard crash in the
//! window between the two `rename`s can leave a mix of old/new binaries. That is
//! tolerable because both binaries come from one release and stay protocol- and
//! schema-compatible; `takd`'s boot-time health check (in the daemon crate) is the
//! backstop for a bad swap that survives to restart.

use std::fs::{self, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::installer::{BinaryArtifact, InstallError, InstallPlan, InstallReport, Installer};
use crate::swap::{Backup, SwapError, back_up, discard, restore, swap_binary_atomically};
use crate::validate::{expected_version_line, probe_binary_version, version_output_matches};

const BINARY_MODE: u32 = 0o755;

/// The production [`Installer`] that writes to the real filesystem.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsInstaller;

impl Installer for FsInstaller {
    fn install(&self, plan: &InstallPlan) -> Result<InstallReport, InstallError> {
        for artifact in &plan.artifacts {
            validate_candidate(artifact, &plan.tag)?;
        }
        commit_all(plan)
    }
}

/// What a successful per-binary commit needs in order to be undone.
enum Committed {
    /// The target pre-existed; restore it from this backup.
    Restored(Backup),
    /// The target was created fresh; remove it on rollback.
    Created(PathBuf),
}

fn validate_candidate(artifact: &BinaryArtifact, tag: &str) -> Result<(), InstallError> {
    let dir = artifact
        .dest
        .parent()
        .ok_or_else(|| InstallError::Swap(SwapError::NoParentDir(artifact.dest.clone())))?;
    let probe_path = stage_probe(dir, &artifact.bytes)
        .map_err(|err| InstallError::Probe(artifact.name.clone(), err.to_string()))?;
    let stdout = probe_binary_version(&probe_path)
        .map_err(|err| InstallError::Probe(artifact.name.clone(), err.to_string()))?;
    drop(probe_path);
    if version_output_matches(&stdout, &artifact.name, tag) {
        Ok(())
    } else {
        Err(InstallError::VersionMismatch {
            name: artifact.name.clone(),
            want: expected_version_line(&artifact.name, tag),
            got: stdout.trim().to_string(),
        })
    }
}

fn stage_probe(dir: &Path, bytes: &[u8]) -> std::io::Result<tempfile::TempPath> {
    let mut file = tempfile::Builder::new()
        .prefix(".tak-update-probe-")
        .tempfile_in(dir)?;
    file.write_all(bytes)?;
    file.flush()?;
    fs::set_permissions(file.path(), Permissions::from_mode(BINARY_MODE))?;
    // Closing the handle (into_temp_path) avoids ETXTBSY when we exec it next.
    Ok(file.into_temp_path())
}

fn commit_all(plan: &InstallPlan) -> Result<InstallReport, InstallError> {
    let mut committed: Vec<Committed> = Vec::new();
    let mut report = InstallReport::default();
    for artifact in &plan.artifacts {
        match commit_one(artifact) {
            Ok(entry) => {
                report.installed.push(artifact.name.clone());
                if let Committed::Restored(backup) = &entry {
                    report.backups.push(backup.path().to_path_buf());
                }
                committed.push(entry);
            }
            Err(err) => {
                let rollback_errors = roll_back(committed);
                return Err(finalize_error(err, rollback_errors));
            }
        }
    }
    Ok(report)
}

fn commit_one(artifact: &BinaryArtifact) -> Result<Committed, InstallError> {
    let backup = back_up(&artifact.dest)?;
    match swap_binary_atomically(&artifact.dest, &artifact.bytes, BINARY_MODE) {
        Ok(()) => Ok(match backup {
            Some(backup) => Committed::Restored(backup),
            None => Committed::Created(artifact.dest.clone()),
        }),
        Err(err) => {
            if let Some(backup) = backup {
                discard(backup);
            }
            Err(err.into())
        }
    }
}

fn roll_back(committed: Vec<Committed>) -> Vec<String> {
    let mut errors = Vec::new();
    for entry in committed.into_iter().rev() {
        match entry {
            Committed::Restored(backup) => {
                if let Err(err) = restore(&backup) {
                    errors.push(err.to_string());
                }
            }
            Committed::Created(path) => {
                if let Err(err) = fs::remove_file(&path) {
                    errors.push(format!("remove {}: {err}", path.display()));
                }
            }
        }
    }
    errors
}

fn finalize_error(original: InstallError, rollback_errors: Vec<String>) -> InstallError {
    if rollback_errors.is_empty() {
        original
    } else {
        InstallError::RollbackFailed {
            original: original.to_string(),
            rollback: rollback_errors.join("; "),
        }
    }
}
