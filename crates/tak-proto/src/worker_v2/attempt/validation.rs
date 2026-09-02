use anyhow::{Result, bail};
use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::v2::{Affinity, OutputSelector, WorkspaceEntryType, WorkspaceManifest};

use super::{
    DispatchAttemptRequest, WorkerAttemptIdentity, WorkerAttemptPayload, WorkerWorkspaceReuse,
    payload_digest,
};
use crate::worker_v2::PROTOCOL_VERSION;

mod tasks;

pub(super) fn validate(request: &DispatchAttemptRequest) -> Result<()> {
    if request.protocol_version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    validate_identity(&request.identity)?;
    if payload_digest(&request.payload)? != request.payload_digest {
        bail!("worker dispatch payload digest mismatch");
    }
    validate_payload(&request.identity, &request.payload)
}

pub(super) fn validate_identity(identity: &WorkerAttemptIdentity) -> Result<()> {
    for value in [
        &identity.run_id,
        &identity.job_id,
        &identity.node_id,
        &identity.fencing_token,
    ] {
        if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            bail!("worker attempt identity is invalid");
        }
    }
    if identity.authored_attempt == 0 || identity.dispatch_generation == 0 {
        bail!("worker attempt generation is invalid");
    }
    Ok(())
}

fn validate_payload(
    identity: &WorkerAttemptIdentity,
    payload: &WorkerAttemptPayload,
) -> Result<()> {
    validate_workspace(payload)?;
    validate_shared_workspace(payload)?;
    tasks::validate(identity, payload)
}

fn validate_shared_workspace(payload: &WorkerAttemptPayload) -> Result<()> {
    if let WorkerWorkspaceReuse::Paths { session_id, paths } = &payload.workspace_reuse {
        validate_shared_session_id(session_id)?;
        if paths.is_empty() {
            bail!("worker paths cache requires at least one selector");
        }
        for selector in paths {
            validate_cache_selector(selector)?;
        }
        return Ok(());
    }
    let WorkerWorkspaceReuse::Shared {
        session_id,
        affinity_group,
    } = &payload.workspace_reuse
    else {
        return Ok(());
    };
    validate_shared_session_id(session_id)?;
    if affinity_group.trim().is_empty() {
        bail!("worker shared affinity group is invalid");
    }
    if payload.tasks.iter().any(|task| {
        !matches!(
            &task.affinity,
            Some(Affinity::RequireSameNode { group }) if group == affinity_group
        )
    }) {
        bail!("worker shared workspace requires matching hard same-node affinity");
    }
    Ok(())
}

fn validate_cache_selector(selector: &OutputSelector) -> Result<()> {
    let value = match selector {
        OutputSelector::Path { value } | OutputSelector::Glob { value } => value,
    };
    if value.is_empty()
        || value.contains('\\')
        || std::path::Path::new(value).components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        bail!("worker paths cache selector escapes the workspace");
    }
    Ok(())
}

fn validate_shared_session_id(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        bail!("worker shared session id is invalid");
    }
    Ok(())
}

fn validate_workspace(payload: &WorkerAttemptPayload) -> Result<()> {
    let descriptor = &payload.workspace.descriptor;
    if !valid_digest(&descriptor.archive_sha256)
        || WorkspaceManifest::new(descriptor.manifest.entries.clone())? != descriptor.manifest
    {
        bail!("worker workspace manifest is not canonical");
    }
    let entries = payload
        .workspace
        .overlays
        .iter()
        .map(|overlay| overlay.entry.clone())
        .collect::<Vec<_>>();
    if WorkspaceManifest::new(entries.clone())?.entries != entries {
        bail!("worker workspace overlays are not canonical");
    }
    for overlay in &payload.workspace.overlays {
        match overlay.entry.entry_type {
            WorkspaceEntryType::File => {
                let content = overlay
                    .content_base64
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("worker file overlay content is missing"))?;
                let content = base64::engine::general_purpose::STANDARD.decode(content)?;
                if content.len() as u64 != overlay.entry.size
                    || format!("{:x}", Sha256::digest(content)) != overlay.entry.content_sha256
                {
                    bail!("worker file overlay content digest mismatch");
                }
            }
            WorkspaceEntryType::Directory | WorkspaceEntryType::Symlink
                if overlay.content_base64.is_none() => {}
            _ => bail!("worker non-file overlay must not carry content"),
        }
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
