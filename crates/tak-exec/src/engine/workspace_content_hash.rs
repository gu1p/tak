use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tak_core::model::{CurrentStateSpec, PathAnchor, build_current_state_manifest};

use super::workspace_collect::collect_workspace_files;

/// Identity of the workspace a remote task would stage: a content hash (the per-job
/// upload-cache key) plus the manifest hash (recorded on the task result).
pub(crate) struct WorkspaceUploadIdentity {
    /// Deterministic, content-sensitive hash (paths + file bytes) — the upload-cache key, so
    /// byte-identical workspaces are uploaded to a node only once.
    pub(crate) content_hash: String,
    /// Paths-only manifest hash, surfaced as `TaskRunResult.context_manifest_hash`. Stable
    /// regardless of whether this task staged or reused a cached upload.
    pub(crate) manifest_hash: String,
}

/// Computes the [`WorkspaceUploadIdentity`] of the workspace a remote task would stage.
///
/// It hashes the SAME logical file set that staging zips — `collect_workspace_files` +
/// `build_current_state_manifest` (already sorted + deduped) — folding each entry's anchor,
/// path, and a SHA-256 of its bytes into the content hash. Deliberately NOT the staged zip's
/// sha256 (the `zip` crate stamps wall-clock mtimes, so it is non-deterministic) nor
/// `manifest.hash` (paths only — different file contents at the same paths collide).
///
/// `state` must be the same `CurrentState` staging uses (e.g. from the session-context-resolved
/// task), so the hashed set matches exactly what would be uploaded.
///
/// ```no_run
/// # // Reason: This helper hashes a real workspace filesystem and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn workspace_upload_identity(
    workspace_root: &Path,
    state: &CurrentStateSpec,
) -> Result<WorkspaceUploadIdentity> {
    let available = collect_workspace_files(workspace_root, state)?;
    let manifest = build_current_state_manifest(available, state);
    let mut hasher = Sha256::new();
    for entry in &manifest.entries {
        if entry.path == "." {
            continue;
        }
        let source = workspace_root.join(&entry.path);
        if !source.is_file() {
            continue;
        }
        let anchor = content_hash_anchor_tag(&entry.anchor);
        hasher.update((anchor.len() as u64).to_be_bytes());
        hasher.update(anchor.as_bytes());
        hasher.update((entry.path.len() as u64).to_be_bytes());
        hasher.update(entry.path.as_bytes());
        let file_digest = hash_file_contents(&source)?;
        hasher.update(file_digest);
    }
    Ok(WorkspaceUploadIdentity {
        content_hash: format!("{:x}", hasher.finalize()),
        manifest_hash: manifest.hash,
    })
}

fn content_hash_anchor_tag(anchor: &PathAnchor) -> String {
    match anchor {
        PathAnchor::Workspace => "workspace".to_string(),
        PathAnchor::Package => "package".to_string(),
        PathAnchor::Repo(name) => format!("repo:{name}"),
    }
}

fn hash_file_contents(path: &Path) -> Result<[u8; 32]> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}
