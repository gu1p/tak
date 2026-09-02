use std::path::Component;

use anyhow::{Result, bail};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceDescriptor, WorkspaceEntry, WorkspaceManifest};

use super::PROTOCOL_VERSION;

const MAX_CACHE_MESSAGE_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCacheProbeRequest {
    pub protocol_version: u16,
    pub descriptor: WorkspaceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCacheUploadRequest {
    pub protocol_version: u16,
    pub descriptor: WorkspaceDescriptor,
    pub archive_base64: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCacheDisposition {
    Hit,
    Miss,
    Stored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCacheResponse {
    pub protocol_version: u16,
    pub workspace_fingerprint: String,
    pub disposition: WorkspaceCacheDisposition,
}

pub fn encode_cache_probe_request(request: &WorkspaceCacheProbeRequest) -> Result<Vec<u8>> {
    validate_version(request.protocol_version)?;
    validate_descriptor(&request.descriptor)?;
    encode_bounded(request)
}

pub fn decode_cache_probe_request(bytes: &[u8]) -> Result<WorkspaceCacheProbeRequest> {
    let request: WorkspaceCacheProbeRequest = decode_bounded(bytes)?;
    validate_version(request.protocol_version)?;
    validate_descriptor(&request.descriptor)?;
    Ok(request)
}

pub fn encode_cache_upload_request(request: &WorkspaceCacheUploadRequest) -> Result<Vec<u8>> {
    validate_upload(request)?;
    encode_bounded(request)
}

pub fn decode_cache_upload_request(bytes: &[u8]) -> Result<WorkspaceCacheUploadRequest> {
    let request: WorkspaceCacheUploadRequest = decode_bounded(bytes)?;
    validate_upload(&request)?;
    Ok(request)
}

pub fn encode_cache_response(response: &WorkspaceCacheResponse) -> Result<Vec<u8>> {
    validate_response(response)?;
    encode_bounded(response)
}

pub fn decode_cache_response(bytes: &[u8]) -> Result<WorkspaceCacheResponse> {
    let response: WorkspaceCacheResponse = decode_bounded(bytes)?;
    validate_response(&response)?;
    Ok(response)
}

fn validate_upload(request: &WorkspaceCacheUploadRequest) -> Result<()> {
    validate_version(request.protocol_version)?;
    validate_descriptor(&request.descriptor)?;
    let archive = base64::engine::general_purpose::STANDARD
        .decode(&request.archive_base64)
        .map_err(|_| anyhow::anyhow!("worker workspace cache archive is not valid base64"))?;
    if archive.len() as u64 != request.descriptor.archive_size
        || format!("{:x}", Sha256::digest(&archive)) != request.descriptor.archive_sha256
    {
        bail!("worker workspace cache archive digest mismatch");
    }
    validate_archive_paths(&archive)
}

fn validate_archive_paths(archive: &[u8]) -> Result<()> {
    for entry in tar::Archive::new(archive).entries()? {
        let entry = entry?;
        let path = entry.path()?;
        if path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            bail!("worker workspace cache archive path escapes its root");
        }
        if entry.header().entry_type().is_symlink() {
            let path = path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("worker workspace cache path is not UTF-8"))?;
            let target = entry
                .link_name()?
                .ok_or_else(|| anyhow::anyhow!("worker workspace cache symlink has no target"))?;
            WorkspaceEntry::symlink(
                path,
                target.to_str().ok_or_else(|| {
                    anyhow::anyhow!("worker workspace cache symlink target is not UTF-8")
                })?,
            )?;
        }
    }
    Ok(())
}

fn validate_descriptor(descriptor: &WorkspaceDescriptor) -> Result<()> {
    if !valid_digest(&descriptor.archive_sha256)
        || WorkspaceManifest::new(descriptor.manifest.entries.clone())? != descriptor.manifest
    {
        bail!("worker workspace cache descriptor is invalid");
    }
    Ok(())
}

fn validate_response(response: &WorkspaceCacheResponse) -> Result<()> {
    validate_version(response.protocol_version)?;
    if !valid_digest(&response.workspace_fingerprint) {
        bail!("worker workspace cache fingerprint is invalid");
    }
    Ok(())
}

fn validate_version(version: u16) -> Result<()> {
    if version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn encode_bounded<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_CACHE_MESSAGE_BYTES {
        bail!("worker workspace cache message exceeds the protocol limit");
    }
    Ok(bytes)
}

fn decode_bounded<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_CACHE_MESSAGE_BYTES {
        bail!("worker workspace cache message exceeds the protocol limit");
    }
    Ok(serde_json::from_slice(bytes)?)
}
