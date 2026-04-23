use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::model::normalize_path_ref;

use super::{StrictRemoteTarget, SyncedOutput};

use super::protocol_result_http::remote_protocol_http_request;

pub(crate) fn sync_remote_outputs(
    staged_root: &Path,
    workspace_root: &Path,
    outputs: &[SyncedOutput],
) -> Result<()> {
    for output in outputs {
        let relative_path = normalized_synced_output_path(output)?;
        let source = staged_root.join(&relative_path);
        if !source.is_file() {
            continue;
        }

        let destination = workspace_root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create output sync directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to sync remote output {} -> {}",
                source.display(),
                destination.display()
            )
        })?;

        let copied_size = fs::metadata(&destination)
            .with_context(|| format!("failed to stat synced output {}", destination.display()))?
            .len();
        if copied_size != output.size_bytes {
            bail!(
                "infra error: remote output {} size mismatch after sync (expected {}, got {})",
                output.path,
                output.size_bytes,
                copied_size
            );
        }
    }

    Ok(())
}

pub(crate) async fn sync_remote_outputs_from_remote(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace_root: &Path,
    outputs: &[SyncedOutput],
) -> Result<()> {
    for output in outputs {
        let relative_path = normalized_synced_output_path(output)?;
        let request_path = build_remote_output_request_path(task_run_id, attempt, &relative_path);
        let (status, response_body) = remote_protocol_http_request(
            target,
            "GET",
            &request_path,
            None,
            "outputs",
            Duration::from_secs(2),
        )
        .await?;
        if status != 200 {
            bail!(
                "infra error: remote node {} output download failed for {} with HTTP {}",
                target.node_id,
                output.path,
                status
            );
        }
        let bytes = response_body;

        let destination = workspace_root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create output sync directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::write(&destination, &bytes).with_context(|| {
            format!(
                "failed to write remote output {} to {}",
                output.path,
                destination.display()
            )
        })?;

        let copied_size = u64::try_from(bytes.len()).unwrap_or(0);
        if copied_size != output.size_bytes {
            bail!(
                "infra error: remote output {} size mismatch after download (expected {}, got {})",
                output.path,
                output.size_bytes,
                copied_size
            );
        }

        let expected_digest = output
            .digest
            .strip_prefix("sha256:")
            .unwrap_or(output.digest.as_str())
            .to_string();
        let actual_digest = format!("{:x}", Sha256::digest(&bytes));
        if actual_digest != expected_digest {
            bail!(
                "infra error: remote output {} digest mismatch after download",
                output.path
            );
        }
    }

    Ok(())
}

fn build_remote_output_request_path(
    task_run_id: &str,
    attempt: u32,
    relative_path: &Path,
) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("attempt", &attempt.to_string());
    query.append_pair("path", &normalize_filesystem_relative_path(relative_path));
    format!("/v1/tasks/{task_run_id}/outputs?{}", query.finish())
}

fn normalized_synced_output_path(output: &SyncedOutput) -> Result<PathBuf> {
    let normalized = normalize_path_ref("workspace", &output.path).map_err(|err| {
        anyhow!(
            "infra error: remote output path `{}` is invalid: {err}",
            output.path
        )
    })?;
    if normalized.path == "." {
        bail!(
            "infra error: remote output path `{}` must reference a file",
            output.path
        );
    }
    Ok(PathBuf::from(normalized.path))
}

pub(crate) fn normalize_filesystem_relative_path(path: &Path) -> String {
    let mut value = String::new();
    for component in path.components() {
        if !value.is_empty() {
            value.push('/');
        }
        value.push_str(&component.as_os_str().to_string_lossy());
    }
    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}
