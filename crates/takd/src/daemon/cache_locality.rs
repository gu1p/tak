use std::fs;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

const PATH_PREFIX: &str = "path-cache:";

pub(super) fn path_content_key(run_id: &str, node_id: &str, session_id: &str) -> Result<String> {
    let identity = serde_json::to_vec(&(run_id, node_id, session_id))?;
    Ok(format!("{PATH_PREFIX}{:x}", Sha256::digest(identity)))
}

pub(super) fn cached_path_content_keys(state_root: &Path) -> Result<Vec<String>> {
    let root = state_root.join("worker-v2-path-caches");
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut keys = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("current").is_file())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|key| valid_digest(key))
        .map(|key| format!("{PATH_PREFIX}{key}"))
        .collect::<Vec<_>>();
    keys.sort();
    Ok(keys)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
