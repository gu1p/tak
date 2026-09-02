use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use tak_proto::worker_v2::DispatchAttemptRequest;

use super::super::workspace_cache::WorkspaceCachePin;
use super::platform::{private_dir, set_executable};
use super::{hash, remove, unpack_verified};
use crate::daemon::{shared_workspace_context, workspace_layer};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SharedBaseIdentity {
    run_id: String,
    node_id: String,
    session_id: String,
    affinity_group: String,
    workspace_fingerprint: String,
    archive_sha256: String,
}

pub(super) fn prepare(
    state_root: PathBuf,
    request: &DispatchAttemptRequest,
    workspace_pin: &WorkspaceCachePin,
    session_id: &str,
    affinity_group: &str,
) -> Result<PathBuf> {
    let identity = identity(request, session_id, affinity_group);
    let fingerprint = &request.payload.workspace.descriptor.manifest.fingerprint;
    let base_root = state_root
        .join("worker-v2-workspace-bases")
        .join(fingerprint);
    let base = workspace_layer::immutable_base(&base_root, |data| {
        unpack_verified(request, workspace_pin, data)
    })?;
    let parent = state_root.join("worker-v2-shared");
    prepare_parent(&parent)?;
    let key = serde_json::to_string(&(&request.identity.run_id, session_id))?;
    let root = parent.join(hash(&key));
    if let Some(data) = existing_data(&root, &identity)? {
        shared_workspace_context::ensure(
            &base,
            &data,
            &root.join("context.json"),
            &request.payload.context_manifest,
        )?;
        return Ok(data);
    }
    publish(&parent, &root, &base, request, &identity)
}

fn publish(
    parent: &Path,
    root: &Path,
    base: &Path,
    request: &DispatchAttemptRequest,
    identity: &SharedBaseIdentity,
) -> Result<PathBuf> {
    let temporary = parent.join(format!("shared-{}.tmp", uuid::Uuid::new_v4()));
    if let Err(error) = initialize(&temporary, base, request, identity) {
        let _ = remove(&temporary);
        return Err(error);
    }
    match fs::rename(&temporary, root) {
        Ok(()) => fs::File::open(parent)?.sync_all()?,
        Err(error) => {
            let existing = existing_data(root, identity);
            remove(&temporary)?;
            return existing?
                .ok_or(error)
                .context("publish worker shared workspace");
        }
    }
    existing_data(root, identity)?
        .ok_or_else(|| anyhow::anyhow!("published worker shared workspace is incomplete"))
}

fn initialize(
    temporary: &Path,
    base: &Path,
    request: &DispatchAttemptRequest,
    identity: &SharedBaseIdentity,
) -> Result<()> {
    private_dir(temporary)?;
    let data = temporary.join("data");
    private_dir(&data)?;
    workspace_layer::private_copy(base, &data)?;
    shared_workspace_context::ensure(
        base,
        &data,
        &temporary.join("context.json"),
        &request.payload.context_manifest,
    )?;
    write_file(
        &temporary.join("identity.json"),
        &serde_json::to_vec(identity)?,
    )?;
    write_file(&temporary.join("ready"), b"v2\n")?;
    fs::File::open(temporary)?.sync_all()?;
    Ok(())
}

fn existing_data(root: &Path, expected: &SharedBaseIdentity) -> Result<Option<PathBuf>> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("inspect worker shared workspace"),
    };
    ensure!(
        metadata.file_type().is_dir(),
        "worker shared workspace is not a directory"
    );
    ensure_regular(&root.join("ready"), "ready marker")?;
    ensure_regular(&root.join("identity.json"), "identity")?;
    let actual =
        serde_json::from_slice::<SharedBaseIdentity>(&fs::read(root.join("identity.json"))?)?;
    ensure!(
        actual == *expected,
        "worker shared workspace base identity mismatch"
    );
    let data = root.join("data");
    let data_metadata = fs::symlink_metadata(&data)?;
    ensure!(
        data_metadata.file_type().is_dir(),
        "worker shared workspace data is not a directory"
    );
    Ok(Some(data))
}

fn ensure_regular(path: &Path, name: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect worker shared workspace {name}"))?;
    ensure!(
        metadata.file_type().is_file(),
        "worker shared workspace {name} is not a regular file"
    );
    Ok(())
}

fn prepare_parent(parent: &Path) -> Result<()> {
    match fs::symlink_metadata(parent) {
        Ok(metadata) => {
            ensure!(
                metadata.file_type().is_dir(),
                "worker shared workspace parent is not a directory"
            );
            private_dir(parent)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => private_dir(parent),
        Err(error) => Err(error.into()),
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    set_executable(path, false)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn identity(
    request: &DispatchAttemptRequest,
    session_id: &str,
    affinity_group: &str,
) -> SharedBaseIdentity {
    SharedBaseIdentity {
        run_id: request.identity.run_id.clone(),
        node_id: request.identity.node_id.clone(),
        session_id: session_id.into(),
        affinity_group: affinity_group.into(),
        workspace_fingerprint: request
            .payload
            .workspace
            .descriptor
            .manifest
            .fingerprint
            .clone(),
        archive_sha256: request.payload.workspace.descriptor.archive_sha256.clone(),
    }
}
