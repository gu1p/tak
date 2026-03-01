use super::*;

pub(super) fn parse_remote_worker_submit_payload(
    payload: &serde_json::Value,
) -> Result<Option<RemoteWorkerSubmitPayload>> {
    let Some(workspace) = payload
        .get("workspace")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(None);
    };
    let mode = workspace
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if mode != "REPO_ZIP_SNAPSHOT" {
        return Ok(None);
    }

    let workspace_zip_base64 = workspace
        .get("archive_zip_base64")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("invalid_submit_fields: missing workspace.archive_zip_base64"))?
        .to_string();

    let execution = payload
        .get("execution")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| anyhow!("invalid_submit_fields: missing execution"))?;
    let steps = execution
        .get("steps")
        .cloned()
        .map(serde_json::from_value::<Vec<StepDef>>)
        .transpose()
        .context("invalid_submit_fields: execution.steps")?
        .unwrap_or_default();
    let timeout_s = execution
        .get("timeout_s")
        .and_then(serde_json::Value::as_u64);
    let runtime = execution
        .get("runtime")
        .filter(|value| !value.is_null())
        .map(parse_remote_worker_runtime_spec)
        .transpose()
        .context("invalid_submit_fields: execution.runtime")?;

    Ok(Some(RemoteWorkerSubmitPayload {
        workspace_zip_base64,
        steps,
        timeout_s,
        runtime,
    }))
}

fn parse_remote_worker_runtime_spec(value: &serde_json::Value) -> Result<RemoteRuntimeSpec> {
    let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) else {
        bail!("execution.runtime.kind is required");
    };
    match kind {
        "containerized" => {
            let image = value
                .get("image")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("execution.runtime.image is required"))?;
            Ok(RemoteRuntimeSpec::Containerized {
                image: image.to_string(),
            })
        }
        _ => bail!("unsupported execution.runtime.kind `{kind}`"),
    }
}
