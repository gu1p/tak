use crate::engine::{RemoteHttpExchangeError, StrictRemoteTarget, transport};

pub(super) fn daemon_timeout(target: &StrictRemoteTarget, phase: &str) -> RemoteHttpExchangeError {
    RemoteHttpExchangeError::timeout(format!(
        "infra error: local takd daemon timed out while contacting remote node {} for {}",
        target.node_id, phase
    ))
}

pub(super) fn daemon_error(
    target: &StrictRemoteTarget,
    err: anyhow::Error,
) -> RemoteHttpExchangeError {
    let message = format!("{err:#}");
    if message.contains("connect_failed") || message.contains("unavailable") {
        return RemoteHttpExchangeError::connect(format!(
            "infra error: remote node {} unavailable via local takd daemon at {}: {message}",
            target.node_id,
            transport::broker_socket_path().display()
        ));
    }
    RemoteHttpExchangeError::other(format!(
        "infra error: local takd daemon rejected request at {} while contacting remote node {}: {message}",
        transport::broker_socket_path().display(),
        target.node_id
    ))
}
