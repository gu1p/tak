use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::normalize_path_ref;

use super::{StrictRemoteTarget, SyncedOutput};

mod remote_download;

use remote_download::{build_remote_output_request_path, download_remote_output};

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
        let destination = workspace_root.join(&relative_path);
        download_remote_output(target, &request_path, &destination, output).await?;
    }

    Ok(())
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
