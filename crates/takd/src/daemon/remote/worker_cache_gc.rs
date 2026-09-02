use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use super::{RemoteNodeContext, SubmitAttemptStore};
use crate::daemon::path_cache::{ACCESS_MARKER, lock::CacheLock};

mod metadata;
mod shared;

use metadata::{accessed, modified_ms};

const WORKSPACE_ROOT: &str = "worker-v2-workspace-cache";
const WORKSPACE_BASE_ROOT: &str = "worker-v2-workspace-bases";
const PATH_ROOT: &str = "worker-v2-path-caches";
const LOCK_ROOT: &str = "worker-v2-cache-locks";

#[derive(Clone, Copy)]
enum Kind {
    Workspace,
    Paths,
}

struct Candidate {
    accessed: u64,
    size: u64,
    path: PathBuf,
    lease_path: PathBuf,
    base_path: Option<PathBuf>,
    kind: Kind,
}

pub(super) fn enforce(context: &RemoteNodeContext, store: &SubmitAttemptStore) -> Result<()> {
    let Some(state_root) = context.state_root() else {
        return Ok(());
    };
    shared::reclaim(&state_root, store)?;
    let mut candidates = workspace_candidates(&state_root)?;
    candidates.extend(path_candidates(&state_root)?);
    let mut total = candidates
        .iter()
        .fold(0_u64, |sum, candidate| sum.saturating_add(candidate.size));
    candidates
        .sort_by(|left, right| (&left.accessed, &left.path).cmp(&(&right.accessed, &right.path)));
    for candidate in candidates {
        if total <= context.runtime_config().worker_cache_budget_bytes() {
            break;
        }
        if evict(&candidate)? {
            total = total.saturating_sub(candidate.size);
        }
    }
    Ok(())
}

pub(in crate::daemon::remote) fn workspace_lock_path(
    state_root: &Path,
    fingerprint: &str,
) -> PathBuf {
    lock_path(state_root, "workspace", fingerprint)
}

pub(in crate::daemon::remote) fn path_lock_path(state_root: &Path, key: &str) -> PathBuf {
    lock_path(state_root, "path", key)
}

fn lock_path(state_root: &Path, kind: &str, key: &str) -> PathBuf {
    state_root
        .join(LOCK_ROOT)
        .join(format!("{kind}-{key}.lock"))
}

fn workspace_candidates(state_root: &Path) -> Result<Vec<Candidate>> {
    let root = state_root.join(WORKSPACE_ROOT);
    let mut candidates = Vec::new();
    for path in children(&root)? {
        let metadata = fs::symlink_metadata(&path)?;
        let Some(fingerprint) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".tar"))
        else {
            continue;
        };
        if !metadata.is_file() || !valid_digest(fingerprint) {
            continue;
        }
        candidates.push(Candidate {
            accessed: modified_ms(&metadata),
            size: metadata.len().saturating_add(
                crate::daemon::workspace_layer::immutable_base_size(
                    &state_root.join(WORKSPACE_BASE_ROOT).join(fingerprint),
                )?,
            ),
            lease_path: workspace_lock_path(state_root, fingerprint),
            base_path: Some(state_root.join(WORKSPACE_BASE_ROOT).join(fingerprint)),
            path,
            kind: Kind::Workspace,
        });
    }
    Ok(candidates)
}

fn path_candidates(state_root: &Path) -> Result<Vec<Candidate>> {
    let root = state_root.join(PATH_ROOT);
    let mut candidates = Vec::new();
    for path in children(&root)? {
        let metadata = fs::symlink_metadata(&path)?;
        let Some(key) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() || !valid_digest(key) {
            continue;
        }
        candidates.push(Candidate {
            accessed: accessed(&path, &metadata),
            size: tree_size(&path)?,
            lease_path: path_lock_path(state_root, key),
            base_path: None,
            path,
            kind: Kind::Paths,
        });
    }
    Ok(candidates)
}

fn evict(candidate: &Candidate) -> Result<bool> {
    let Some(lease) = CacheLock::try_acquire(&candidate.lease_path)? else {
        return Ok(false);
    };
    match candidate.kind {
        Kind::Workspace => crate::daemon::workspace_layer::evict_pair(
            &candidate.path,
            candidate
                .base_path
                .as_deref()
                .ok_or_else(|| anyhow!("missing workspace base"))?,
        ),
        Kind::Paths => {
            let Some(cache) = CacheLock::try_acquire(&candidate.path.join("cache.lock"))? else {
                return Ok(false);
            };
            let tombstone = candidate
                .path
                .with_file_name(format!(".gc-{}.tmp", uuid::Uuid::new_v4()));
            match fs::rename(&candidate.path, &tombstone) {
                Ok(()) => {
                    drop(cache);
                    drop(lease);
                    fs::remove_dir_all(tombstone)?;
                    Ok(true)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error.into()),
            }
        }
    }
}

fn children(root: &Path) -> Result<Vec<PathBuf>> {
    match fs::read_dir(root) {
        Ok(entries) => Ok(entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

fn tree_size(root: &Path) -> Result<u64> {
    let mut size = 0_u64;
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        for child in children(&path)? {
            let metadata = fs::symlink_metadata(&child)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                pending.push(child);
            } else if metadata.is_file()
                && child.file_name().and_then(|name| name.to_str()) != Some(ACCESS_MARKER)
            {
                size = size.saturating_add(metadata.len());
            }
        }
    }
    Ok(size)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
