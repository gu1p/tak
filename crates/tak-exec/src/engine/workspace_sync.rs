fn sync_remote_outputs(
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

async fn sync_remote_outputs_from_remote(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace_root: &Path,
    outputs: &[SyncedOutput],
) -> Result<()> {
    for output in outputs {
        let relative_path = normalized_synced_output_path(output)?;
        let path_query = relative_path.to_string_lossy().to_string();
        let request_path =
            format!("/v1/tasks/{task_run_id}/outputs?attempt={attempt}&path={path_query}");
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

        let parsed: serde_json::Value =
            serde_json::from_str(&response_body).with_context(|| {
                format!(
                    "infra error: remote node {} output download payload is invalid JSON for {}",
                    target.node_id, output.path
                )
            })?;
        let encoded = parsed
            .get("data_base64")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} output download payload is missing data_base64 for {}",
                    target.node_id,
                    output.path
                )
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .with_context(|| {
                format!(
                    "infra error: remote node {} output download payload has invalid base64 for {}",
                    target.node_id, output.path
                )
            })?;

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

fn normalize_filesystem_relative_path(path: &Path) -> String {
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
