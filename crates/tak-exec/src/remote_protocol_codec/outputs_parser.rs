pub(crate) fn parse_remote_result_outputs(
    target: &StrictRemoteTarget,
    result: &serde_json::Value,
) -> Result<Vec<SyncedOutput>> {
    let Some(outputs) = result.get("outputs") else {
        return Ok(Vec::new());
    };
    let outputs = outputs.as_array().ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} result outputs field must be an array",
            target.node_id
        )
    })?;

    let mut synced_outputs = Vec::with_capacity(outputs.len());
    for output in outputs {
        let path = output
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string path",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if path.is_empty() {
            bail!(
                "infra error: remote node {} result output path cannot be empty",
                target.node_id
            );
        }

        let digest = output
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string digest",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if digest.is_empty() {
            bail!(
                "infra error: remote node {} result output digest cannot be empty",
                target.node_id
            );
        }

        let size_bytes = output
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing numeric size",
                    target.node_id
                )
            })?;

        synced_outputs.push(SyncedOutput {
            path,
            digest,
            size_bytes,
        });
    }

    Ok(synced_outputs)
}
