use std::time::Duration;

use prost::Message;
use tak_proto::NodeInfo;

use super::StrictRemoteTarget;

use super::preflight_failure::RemoteNodeInfoFailure;
use super::protocol_result_http::remote_protocol_http_request;

/// Probes the remote node info endpoint and confirms V1 protobuf support.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn detect_remote_protocol_mode(
    target: &StrictRemoteTarget,
) -> std::result::Result<NodeInfo, RemoteNodeInfoFailure> {
    let (status, body) = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/info",
        None,
        "node info",
        Duration::from_millis(250),
    )
    .await
    .map_err(RemoteNodeInfoFailure::from_http_exchange)?;
    if status == 401 || status == 403 {
        return Err(RemoteNodeInfoFailure::auth(format!(
            "infra error: remote node {} auth failed during node info with HTTP {}",
            target.node_id, status
        )));
    }
    if status != 200 {
        return Err(RemoteNodeInfoFailure::http_status(format!(
            "infra error: remote node {} node info probe failed with HTTP {}",
            target.node_id, status
        )));
    }

    let node = NodeInfo::decode(body.as_slice()).map_err(|_| {
        RemoteNodeInfoFailure::invalid_metadata(format!(
            "infra error: remote node {} returned invalid protobuf for node info",
            target.node_id
        ))
    })?;
    Ok(node)
}
