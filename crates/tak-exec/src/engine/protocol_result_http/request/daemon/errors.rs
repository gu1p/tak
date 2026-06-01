use crate::engine::{RemoteHttpExchangeError, StrictRemoteTarget, transport};

pub(super) fn daemon_timeout(target: &StrictRemoteTarget, phase: &str) -> RemoteHttpExchangeError {
    if target.is_daemon_tor_placement() {
        return RemoteHttpExchangeError::timeout(format!(
            "infra error: local takd daemon timed out during {phase}\n\nsubsystem: local_daemon\nstage: remote placement\ntransport: tor\nretryable: yes\nsource: {}:{}",
            file!(),
            line!()
        ));
    }
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
    if target.is_daemon_tor_placement() {
        return daemon_placement_error(message);
    }
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

fn daemon_placement_error(original_error: String) -> RemoteHttpExchangeError {
    let retryable = if original_error.contains("No known remote worker satisfies")
        || original_error.contains("resource_requirements_exceed_worker_capacity")
    {
        "no"
    } else {
        "yes"
    };
    RemoteHttpExchangeError::other(format!(
        "infra error: local takd could not place this task on a Tor remote worker\n\nsubsystem: placement\nstage: remote placement\ntransport: tor\nretryable: {retryable}\noriginal_error:\n{original_error}\nsource: {}:{}",
        file!(),
        line!()
    ))
}

#[cfg(test)]
mod errors_tests;
