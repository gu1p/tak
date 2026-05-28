use http_body_util::BodyExt;

use super::super::super::{RemoteHttpExchangeError, StrictRemoteTarget, transport};
use super::{RemoteHttpResponse, ResponseHeader};

pub(super) async fn read_response_with_headers(
    target: &StrictRemoteTarget,
    phase: &str,
    response: hyper::Response<hyper::body::Incoming>,
    use_broker: bool,
) -> std::result::Result<RemoteHttpResponse, RemoteHttpExchangeError> {
    let status = response.status().as_u16();
    let broker_error = response
        .headers()
        .get("X-Tak-Broker-Error")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            Some(ResponseHeader {
                name: name.as_str().to_string(),
                value: value.to_str().ok()?.to_string(),
            })
        })
        .collect();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|_| truncated_response(target, phase))?
        .to_bytes()
        .to_vec();
    if use_broker && let Some(code) = broker_error {
        return Err(broker_error_response(target, &body, &code, status));
    }
    Ok(RemoteHttpResponse {
        status,
        headers,
        body,
        daemon_task_handle: None,
        daemon_peer_node_id: None,
        daemon_peer_endpoint: None,
    })
}

pub(super) fn broker_error_response(
    target: &StrictRemoteTarget,
    body: &[u8],
    code: &str,
    status: u16,
) -> RemoteHttpExchangeError {
    let detail = String::from_utf8_lossy(body);
    if status == 502 {
        return RemoteHttpExchangeError::connect(format!(
            "infra error: remote node {} unavailable via local takd Tor broker at {}: {} ({})",
            target.node_id,
            transport::broker_socket_path().display(),
            detail,
            code
        ));
    }
    RemoteHttpExchangeError::other(format!(
        "infra error: local takd Tor broker rejected request at {} while contacting remote node {}: {} ({})",
        transport::broker_socket_path().display(),
        target.node_id,
        detail,
        code
    ))
}

pub(super) fn malformed_response(
    target: &StrictRemoteTarget,
    phase: &str,
) -> RemoteHttpExchangeError {
    RemoteHttpExchangeError::other(format!(
        "infra error: remote node {} returned malformed HTTP response for {}",
        target.node_id, phase
    ))
}

fn truncated_response(target: &StrictRemoteTarget, phase: &str) -> RemoteHttpExchangeError {
    RemoteHttpExchangeError::other(format!(
        "infra error: remote node {} returned truncated HTTP body for {}",
        target.node_id, phase
    ))
}

pub(super) fn timeout_error(target: &StrictRemoteTarget, phase: &str) -> RemoteHttpExchangeError {
    RemoteHttpExchangeError::timeout(format!(
        "infra error: remote node {} at {} via {} {} request timed out",
        target.node_id,
        target.endpoint,
        target.transport_kind.as_result_value(),
        phase
    ))
}

#[path = "response_tests.rs"]
mod tests;
