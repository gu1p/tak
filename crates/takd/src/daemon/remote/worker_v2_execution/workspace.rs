use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntryType;
use tak_proto::worker_v2::{DispatchAttemptRequest, WorkerWorkspaceOverlay, WorkerWorkspaceReuse};

use super::super::{RemoteNodeContext, worker_cache_gc};
use super::workspace_cache::WorkspaceCachePin;
use crate::daemon::path_cache::{PathCache, Publication, Snapshot};

mod manifest;
mod platform;
mod private;
mod shared;

use platform::{create_symlink, private_dir, set_executable};

pub(super) struct PreparedWorkspace {
    pub(super) workspace_root: PathBuf,
    pub(super) home: PathBuf,
    pub(super) temporary: PathBuf,
    path_cache: Option<(PathCache, Snapshot)>,
}

pub(super) fn prepare(
    context: &RemoteNodeContext,
    request: &DispatchAttemptRequest,
    workspace_pin: &WorkspaceCachePin,
) -> Result<PreparedWorkspace> {
    let state_root = required_state_root(context)?;
    let attempt_root = attempt_root(&state_root, request);
    private_dir(&attempt_root)?;
    let (workspace_root, path_cache) = match &request.payload.workspace_reuse {
        WorkerWorkspaceReuse::Private => (
            private::prepare(&state_root, request, workspace_pin, &attempt_root)?,
            None,
        ),
        WorkerWorkspaceReuse::Paths { session_id, paths } => {
            let workspace = private::prepare(&state_root, request, workspace_pin, &attempt_root)?;
            let key = serde_json::to_string(&(
                &request.identity.run_id,
                &request.identity.node_id,
                session_id,
            ))?;
            let cache_key = hash(&key);
            let cache = PathCache::new_leased(
                state_root.join("worker-v2-path-caches").join(&cache_key),
                paths.clone(),
                &worker_cache_gc::path_lock_path(&state_root, &cache_key),
            )?;
            let generation = cache.restore_into(&workspace)?;
            (workspace, Some((cache, generation)))
        }
        WorkerWorkspaceReuse::Shared {
            session_id,
            affinity_group,
        } => (
            shared::prepare(
                state_root,
                request,
                workspace_pin,
                session_id,
                affinity_group,
            )?,
            None,
        ),
    };
    for overlay in &request.payload.workspace.overlays {
        apply_overlay(&workspace_root, overlay)?;
    }
    let home = attempt_root.join("home");
    let temporary = attempt_root.join("tmp");
    private_dir(&home)?;
    private_dir(&temporary)?;
    Ok(PreparedWorkspace {
        workspace_root,
        home,
        temporary,
        path_cache,
    })
}
pub(super) fn cleanup_attempt(
    context: &RemoteNodeContext,
    request: &DispatchAttemptRequest,
) -> Result<()> {
    let state_root = required_state_root(context)?;
    remove(&attempt_root(&state_root, request))
}
impl PreparedWorkspace {
    pub(super) fn publish_path_cache(&self) -> Result<Option<Publication>> {
        self.path_cache
            .as_ref()
            .map(|(cache, generation)| cache.publish_from(&self.workspace_root, *generation))
            .transpose()
    }
}
pub(super) fn unpack_verified(
    request: &DispatchAttemptRequest,
    workspace_pin: &WorkspaceCachePin,
    root: &Path,
) -> Result<()> {
    let archive = workspace_pin.read_verified()?;
    tar::Archive::new(archive.as_slice())
        .unpack(root)
        .context("unpack worker v2 workspace")?;
    let actual = manifest::scan(root)?;
    if actual != request.payload.workspace.descriptor.manifest {
        bail!("worker workspace archive does not match its canonical manifest");
    }
    Ok(())
}

pub(super) fn filter_context(request: &DispatchAttemptRequest, root: &Path) -> Result<()> {
    let allowed = request
        .payload
        .context_manifest
        .paths
        .iter()
        .collect::<BTreeSet<_>>();
    for entry in request
        .payload
        .workspace
        .descriptor
        .manifest
        .entries
        .iter()
        .rev()
        .filter(|entry| !allowed.contains(&entry.path))
    {
        remove(&root.join(&entry.path))?;
    }
    Ok(())
}

fn apply_overlay(root: &Path, overlay: &WorkerWorkspaceOverlay) -> Result<()> {
    let path = root.join(&overlay.entry.path);
    remove(&path)?;
    if let Some(parent) = path.parent() {
        private_dir(parent)?;
    }
    match overlay.entry.entry_type {
        WorkspaceEntryType::Directory => private_dir(&path),
        WorkspaceEntryType::File => {
            let bytes = base64::engine::general_purpose::STANDARD.decode(
                overlay
                    .content_base64
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("worker file overlay content is missing"))?,
            )?;
            fs::write(&path, bytes)?;
            set_executable(&path, overlay.entry.executable)
        }
        WorkspaceEntryType::Symlink => create_symlink(
            overlay.entry.symlink_target.as_deref().unwrap_or_default(),
            &path,
        ),
    }
}

pub(super) fn remove(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub(super) fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn required_state_root(context: &RemoteNodeContext) -> Result<PathBuf> {
    context
        .state_root()
        .ok_or_else(|| anyhow::anyhow!("worker v2 execution requires a state root"))
}

fn attempt_root(state_root: &Path, request: &DispatchAttemptRequest) -> PathBuf {
    state_root
        .join("worker-v2-attempts")
        .join(hash(&request.identity.fencing_token))
}
