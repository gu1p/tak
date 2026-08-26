use crate::engine::{RemoteHttpExchangeError, StrictRemoteTarget, transport};

#[path = "errors/local_error.rs"]
mod local_error;
pub(super) use local_error::DaemonLocalError;

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
    let failed_node_id = daemon_peer_error(&err).map(|error| error.node_id.clone());
    let original_error = format!("{err:#}");
    let exchange_error = if let Some(local_error) = daemon_local_error(&err) {
        local_error_to_exchange_error(target, local_error)
    } else if target.is_daemon_tor_placement() {
        RemoteHttpExchangeError::other(placement_error_message(
            &DaemonLocalError::response(original_error, None, Some(false)),
            false,
        ))
    } else {
        RemoteHttpExchangeError::other(format!(
            "infra error: local takd daemon rejected request at {} while contacting remote node {}: {err:#}",
            transport::broker_socket_path().display(),
            target.node_id
        ))
    };
    if let Some(node_id) = failed_node_id {
        return exchange_error.with_failed_node_id(node_id);
    }
    exchange_error
}

#[derive(Debug)]
struct DaemonPeerError {
    node_id: String,
    source: anyhow::Error,
}

impl std::fmt::Display for DaemonPeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.source)
    }
}

impl std::error::Error for DaemonPeerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

pub(super) fn peer_error(node_id: &str, source: anyhow::Error) -> anyhow::Error {
    DaemonPeerError {
        node_id: node_id.to_string(),
        source,
    }
    .into()
}

fn daemon_peer_error(err: &anyhow::Error) -> Option<&DaemonPeerError> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<DaemonPeerError>())
}

fn daemon_local_error(err: &anyhow::Error) -> Option<&DaemonLocalError> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<DaemonLocalError>())
}

fn local_error_to_exchange_error(
    target: &StrictRemoteTarget,
    error: &DaemonLocalError,
) -> RemoteHttpExchangeError {
    match error {
        DaemonLocalError::Response { retryable, .. } => {
            response_error_to_exchange_error(target, error, retryable.unwrap_or(false))
        }
        DaemonLocalError::Connect { message } => RemoteHttpExchangeError::connect(message.clone()),
        DaemonLocalError::RetryableClient { message } => {
            retryable_client_error(target, message.clone())
        }
    }
}

fn response_error_to_exchange_error(
    target: &StrictRemoteTarget,
    error: &DaemonLocalError,
    is_retryable: bool,
) -> RemoteHttpExchangeError {
    if target.is_daemon_tor_placement() {
        return daemon_placement_error(error, is_retryable);
    }
    let message = daemon_response_error_message(target, error, is_retryable);
    if is_retryable {
        RemoteHttpExchangeError::retryable_other(message)
    } else {
        RemoteHttpExchangeError::other(message)
    }
}

fn daemon_placement_error(error: &DaemonLocalError, is_retryable: bool) -> RemoteHttpExchangeError {
    let message = placement_error_message(error, is_retryable);
    if is_retryable {
        return RemoteHttpExchangeError::retryable_other(message);
    }
    RemoteHttpExchangeError::other(message)
}

fn retryable_client_error(target: &StrictRemoteTarget, message: String) -> RemoteHttpExchangeError {
    if target.is_daemon_tor_placement() {
        return RemoteHttpExchangeError::retryable_other(placement_error_message(
            &DaemonLocalError::response(message, None, Some(true)),
            true,
        ));
    }
    RemoteHttpExchangeError::retryable_other(message)
}

fn daemon_response_error_message(
    target: &StrictRemoteTarget,
    error: &DaemonLocalError,
    is_retryable: bool,
) -> String {
    let mut lines = vec![
        format!(
            "infra error: local takd daemon rejected request at {} while contacting remote node {}",
            transport::broker_socket_path().display(),
            target.node_id
        ),
        String::new(),
        "subsystem: local_daemon".to_string(),
        format!("retryable: {}", yes_no(is_retryable)),
    ];
    push_error_metadata(&mut lines, error);
    lines.push(format!("source: {}:{}", file!(), line!()));
    lines.join("\n")
}

fn placement_error_message(error: &DaemonLocalError, is_retryable: bool) -> String {
    let mut lines = vec![
        "infra error: local takd could not place this task on a Tor remote worker".to_string(),
        String::new(),
        "subsystem: placement".to_string(),
        "stage: remote placement".to_string(),
        "transport: tor".to_string(),
        format!("retryable: {}", yes_no(is_retryable)),
    ];
    push_error_metadata(&mut lines, error);
    lines.push(format!("source: {}:{}", file!(), line!()));
    lines.join("\n")
}

fn push_error_metadata(lines: &mut Vec<String>, error: &DaemonLocalError) {
    if let DaemonLocalError::Response {
        code,
        retryable,
        message,
    } = error
    {
        if let Some(code) = code.as_deref() {
            lines.push(format!("code: {code}"));
        }
        lines.push("original_error:".to_string());
        lines.push(message.clone());
        if retryable.is_none() {
            lines.push(
                "diagnostic: local takd daemon response did not include structured retryability metadata; restart/update local takd."
                    .to_string(),
            );
        }
        return;
    }
    lines.push("original_error:".to_string());
    lines.push(error.message().to_string());
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod errors_tests;
