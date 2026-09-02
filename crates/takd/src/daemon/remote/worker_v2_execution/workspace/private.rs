use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_proto::worker_v2::DispatchAttemptRequest;

use super::super::workspace_cache::WorkspaceCachePin;
use crate::daemon::workspace_layer;

pub(super) fn prepare(
    state_root: &Path,
    request: &DispatchAttemptRequest,
    workspace_pin: &WorkspaceCachePin,
    attempt_root: &Path,
) -> Result<PathBuf> {
    let root = attempt_root.join("workspace");
    if root.try_exists()? {
        return Ok(root);
    }
    let fingerprint = &request.payload.workspace.descriptor.manifest.fingerprint;
    let base_root = state_root
        .join("worker-v2-workspace-bases")
        .join(fingerprint);
    let base = workspace_layer::immutable_base(&base_root, |data| {
        super::unpack_verified(request, workspace_pin, data)
    })?;
    workspace_layer::private_copy(&base, &root)?;
    super::filter_context(request, &root)?;
    Ok(root)
}
