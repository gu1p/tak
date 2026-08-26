use super::super::RemoteHttpExchangeError;
use super::super::remote_models::StrictRemoteTarget;
use super::super::remote_submit_failure::RemoteSubmitFailure;

pub(super) fn submit_transport_error(err: RemoteHttpExchangeError) -> RemoteSubmitFailure {
    let failed_node_id = err.failed_node_id().map(str::to_string);
    let mut failure = if err.is_retryable() {
        RemoteSubmitFailure::retryable_other(err.to_string())
    } else {
        RemoteSubmitFailure::other(err.to_string())
    };
    if let Some(node_id) = failed_node_id {
        failure = failure.with_failed_node_id(node_id);
    }
    failure
}

pub(super) fn submit_protocol_error(
    target: &StrictRemoteTarget,
    phase: &str,
    status: u16,
) -> RemoteSubmitFailure {
    let message = format!(
        "infra error: remote node {} {} failed with HTTP {}",
        target.node_id, phase, status
    );
    match status {
        401 | 403 => RemoteSubmitFailure::auth(message),
        _ => RemoteSubmitFailure::other(message),
    }
}

pub(super) fn submit_decode_error(target: &StrictRemoteTarget, phase: &str) -> RemoteSubmitFailure {
    RemoteSubmitFailure::other(format!(
        "infra error: remote node {} returned invalid protobuf for {}",
        target.node_id, phase
    ))
}
