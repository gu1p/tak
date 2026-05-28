use super::super::RemoteHttpExchangeError;
use super::super::remote_models::StrictRemoteTarget;
use super::super::remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind};

pub(super) fn submit_transport_error(err: RemoteHttpExchangeError) -> RemoteSubmitFailure {
    RemoteSubmitFailure {
        kind: RemoteSubmitFailureKind::Other,
        message: err.to_string(),
    }
}

pub(super) fn submit_protocol_error(
    target: &StrictRemoteTarget,
    phase: &str,
    status: u16,
) -> RemoteSubmitFailure {
    RemoteSubmitFailure {
        kind: match status {
            401 | 403 => RemoteSubmitFailureKind::Auth,
            _ => RemoteSubmitFailureKind::Other,
        },
        message: format!(
            "infra error: remote node {} {} failed with HTTP {}",
            target.node_id, phase, status
        ),
    }
}

pub(super) fn submit_decode_error(target: &StrictRemoteTarget, phase: &str) -> RemoteSubmitFailure {
    RemoteSubmitFailure {
        kind: RemoteSubmitFailureKind::Other,
        message: format!(
            "infra error: remote node {} returned invalid protobuf for {}",
            target.node_id, phase
        ),
    }
}
