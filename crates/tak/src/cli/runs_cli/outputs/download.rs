use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use tak_proto::local_daemon::v2::{MAX_WORKSPACE_CHUNK_BYTES, Operation, OutputArtifact, Response};

pub(super) async fn file(socket: &Path, artifact: &OutputArtifact, path: &Path) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut digest = Sha256::new();
    let mut offset = 0_u64;
    while offset < artifact.size {
        let response = super::super::request(
            socket,
            "tak-runs-output-chunk",
            Operation::GetOutputChunk {
                artifact_id: artifact.artifact_id.clone(),
                offset,
                max_bytes: MAX_WORKSPACE_CHUNK_BYTES as u32,
            },
            false,
        )
        .await?;
        let Response::OutputChunk {
            artifact_id,
            offset: response_offset,
            chunk_base64,
            complete,
            ..
        } = response
        else {
            bail!(super::super::MISMATCH_DIAGNOSTIC)
        };
        let encoded_limit = MAX_WORKSPACE_CHUNK_BYTES.div_ceil(3) * 4;
        if chunk_base64.len() > encoded_limit {
            bail!("daemon output chunk is invalid");
        }
        let chunk = STANDARD
            .decode(chunk_base64)
            .map_err(|_| anyhow::anyhow!("daemon output chunk is invalid"))?;
        let next_offset = offset.checked_add(chunk.len() as u64);
        if artifact_id != artifact.artifact_id
            || response_offset != offset
            || chunk.is_empty()
            || chunk.len() > MAX_WORKSPACE_CHUNK_BYTES
            || next_offset.is_none_or(|next| next > artifact.size)
        {
            bail!("daemon output chunk progress is invalid");
        }
        let next_offset = next_offset.expect("validated output offset");
        if complete != (next_offset == artifact.size) {
            bail!("daemon output artifact completion is invalid");
        }
        file.write_all(&chunk)?;
        digest.update(&chunk);
        offset = next_offset;
    }
    file.sync_all()?;
    if format!("{:x}", digest.finalize()) != artifact.sha256 {
        bail!("daemon output artifact digest mismatch");
    }
    set_executable(&file, artifact.executable)
}

#[cfg(unix)]
fn set_executable(file: &File, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if executable { 0o755 } else { 0o644 };
    file.set_permissions(std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_file: &File, _executable: bool) -> Result<()> {
    Ok(())
}
