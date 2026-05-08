use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message;
use tak_proto::CancelTaskResponse;

use super::StrictRemoteTarget;
use super::protocol_result_http::remote_protocol_http_request;

pub(crate) async fn remote_protocol_cancel(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
) -> Result<()> {
    let path = format!("/v1/tasks/{task_run_id}/cancel?attempt={attempt}");
    let (status, body) = remote_protocol_http_request(
        target,
        "POST",
        &path,
        None,
        "cancel",
        Duration::from_secs(5),
    )
    .await
    .map_err(|err| anyhow::anyhow!("{err}"))?;
    if status != 202 {
        bail!(
            "infra error: remote node {} cancel failed with HTTP {}",
            target.node_id,
            status
        );
    }
    let response = CancelTaskResponse::decode(body.as_slice()).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for cancel",
            target.node_id
        )
    })?;
    if !response.cancelled {
        bail!(
            "infra error: remote node {} reported no active task to cancel for {}",
            target.node_id,
            task_run_id
        );
    }
    Ok(())
}
