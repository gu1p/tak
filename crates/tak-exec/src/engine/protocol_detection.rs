use prost::Message;
use tak_proto::NodeInfo;

/// Probes the remote node info endpoint and confirms V1 protobuf support.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn detect_remote_protocol_mode(target: &StrictRemoteTarget) -> Result<NodeInfo> {
    let (status, body) = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/info",
        None,
        "node info",
        Duration::from_millis(250),
    )
    .await?;
    if status == 401 || status == 403 {
        bail!(
            "infra error: remote node {} auth failed during node info with HTTP {}",
            target.node_id,
            status
        );
    }
    if status != 200 {
        bail!(
            "infra error: remote node {} node info probe failed with HTTP {}",
            target.node_id,
            status
        );
    }

    let node = NodeInfo::decode(body.as_slice()).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for node info",
            target.node_id
        )
    })?;
    Ok(node)
}
