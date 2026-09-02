use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::children;
use crate::daemon::remote::SubmitAttemptStore;

const ROOT: &str = "worker-v2-shared";

#[derive(Deserialize)]
struct Identity {
    run_id: String,
    session_id: String,
}

pub(super) fn reclaim(state_root: &Path, store: &SubmitAttemptStore) -> Result<()> {
    let root = state_root.join(ROOT);
    let terminal = store.terminal_worker_v2_runs()?;
    for path in children(&root)? {
        let Some(identity) = canonical_identity(&path)? else {
            continue;
        };
        if terminal.contains(&identity.run_id) {
            store.reclaim_terminal_worker_v2_run(&identity.run_id, || evict(&root, &path))?;
        }
    }
    Ok(())
}

fn canonical_identity(path: &Path) -> Result<Option<Identity>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Ok(None);
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    let identity_path = path.join("identity.json");
    let identity_metadata = match fs::symlink_metadata(&identity_path) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if identity_metadata.file_type().is_symlink() {
        return Ok(None);
    }
    let identity = match serde_json::from_slice::<Identity>(&fs::read(identity_path)?) {
        Ok(identity) => identity,
        Err(_) => return Ok(None),
    };
    if !valid(&identity.run_id) || !valid(&identity.session_id) {
        return Ok(None);
    }
    let key = serde_json::to_string(&(&identity.run_id, &identity.session_id))?;
    let expected = format!("{:x}", Sha256::digest(key));
    Ok((name == expected).then_some(identity))
}

fn valid(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

fn evict(root: &Path, path: &Path) -> Result<()> {
    ensure!(
        path.parent() == Some(root),
        "shared workspace escapes worker root"
    );
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "shared workspace is not a directory"
    );
    let tombstone: PathBuf = root.join(format!(".gc-shared-{}.tmp", uuid::Uuid::new_v4()));
    fs::rename(path, &tombstone)?;
    fs::remove_dir_all(tombstone)?;
    Ok(())
}
