use std::fs;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};
use tak_loader::V2AuthoredRoot;

pub(super) fn worktree_scope_key(root: &V2AuthoredRoot) -> Result<String> {
    scope_key_for_path(&root.workspace_root)
}

pub(super) fn scope_key_for_path(path: &Path) -> Result<String> {
    let canonical = fs::canonicalize(path)?;
    let digest = Sha256::digest(canonical.as_os_str().as_encoded_bytes());
    Ok(format!("worktree-{digest:x}"))
}
