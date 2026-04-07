pub(crate) fn parse_remote_result_outputs(
    target: &StrictRemoteTarget,
    result: &GetTaskResultResponse,
) -> Result<Vec<SyncedOutput>> {
    let mut synced_outputs = Vec::with_capacity(result.outputs.len());
    for output in &result.outputs {
        let path = output.path.trim().to_string();
        if path.is_empty() {
            bail!(
                "infra error: remote node {} result output path cannot be empty",
                target.node_id
            );
        }
        let digest = output.digest.trim().to_string();
        if digest.is_empty() {
            bail!(
                "infra error: remote node {} result output digest cannot be empty",
                target.node_id
            );
        }
        synced_outputs.push(SyncedOutput {
            path,
            digest,
            size_bytes: output.size_bytes,
        });
    }
    Ok(synced_outputs)
}
