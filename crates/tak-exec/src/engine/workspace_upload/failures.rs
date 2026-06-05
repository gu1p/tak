use super::super::RemoteHttpExchangeError;
use super::super::remote_models::StrictRemoteTarget;
use super::super::remote_submit_failure::RemoteSubmitFailure;

pub(super) fn submit_transport_error(err: RemoteHttpExchangeError) -> RemoteSubmitFailure {
    if err.is_retryable() {
        return RemoteSubmitFailure::retryable_other(err.to_string());
    }
    RemoteSubmitFailure::other(err.to_string())
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
