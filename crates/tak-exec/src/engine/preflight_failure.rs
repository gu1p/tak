#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteNodeInfoFailureKind {
    Timeout,
    Auth,
    HttpStatus,
    InvalidMetadata,
    Connect,
    Other,
}

#[derive(Debug, Clone)]
struct RemoteNodeInfoFailure {
    kind: RemoteNodeInfoFailureKind,
    message: String,
}

impl std::fmt::Display for RemoteNodeInfoFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteNodeInfoFailure {}

impl RemoteNodeInfoFailure {
    fn timeout(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::Timeout,
            message,
        }
    }

    fn auth(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::Auth,
            message,
        }
    }

    fn http_status(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::HttpStatus,
            message,
        }
    }

    fn invalid_metadata(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::InvalidMetadata,
            message,
        }
    }

    fn connect(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::Connect,
            message,
        }
    }

    fn other(message: String) -> Self {
        Self {
            kind: RemoteNodeInfoFailureKind::Other,
            message,
        }
    }

    fn from_http_exchange(err: RemoteHttpExchangeError) -> Self {
        match err.kind {
            RemoteHttpExchangeErrorKind::Timeout => Self::timeout(err.message),
            RemoteHttpExchangeErrorKind::Connect => Self::connect(err.message),
            RemoteHttpExchangeErrorKind::Other => Self::other(err.message),
        }
    }
}

fn remote_preflight_timeout_failure(
    target: &StrictRemoteTarget,
    message: String,
) -> RemotePreflightFailure {
    RemotePreflightFailure {
        node_id: target.node_id.clone(),
        endpoint: target.endpoint.clone(),
        transport: target.transport_kind.as_result_value().to_string(),
        kind: RemotePreflightFailureKind::Timeout,
        message,
        live_transport_state: None,
        live_transport_detail: None,
        last_observation: load_remote_observation(&target.node_id).ok().flatten(),
    }
}

fn remote_preflight_error_failure(
    target: &StrictRemoteTarget,
    failure: RemoteNodeInfoFailure,
) -> RemotePreflightFailure {
    let kind = classify_preflight_failure_kind(failure.kind);
    let last_observation = matches!(
        kind,
        RemotePreflightFailureKind::Timeout | RemotePreflightFailureKind::Connect
    )
    .then(|| load_remote_observation(&target.node_id).ok().flatten())
    .flatten();
    RemotePreflightFailure {
        node_id: target.node_id.clone(),
        endpoint: target.endpoint.clone(),
        transport: target.transport_kind.as_result_value().to_string(),
        kind,
        message: failure.message,
        live_transport_state: None,
        live_transport_detail: None,
        last_observation,
    }
}

fn remote_preflight_unhealthy_failure(
    target: &StrictRemoteTarget,
    node: &tak_proto::NodeInfo,
) -> RemotePreflightFailure {
    RemotePreflightFailure {
        node_id: target.node_id.clone(),
        endpoint: target.endpoint.clone(),
        transport: target.transport_kind.as_result_value().to_string(),
        kind: RemotePreflightFailureKind::Unhealthy,
        message: format!(
            "infra error: remote node {} reported transport state {} at {}",
            target.node_id, node.transport_state, target.endpoint
        ),
        live_transport_state: Some(node.transport_state.clone()),
        live_transport_detail: (!node.transport_detail.is_empty())
            .then_some(node.transport_detail.clone()),
        last_observation: None,
    }
}

fn classify_preflight_failure_kind(kind: RemoteNodeInfoFailureKind) -> RemotePreflightFailureKind {
    match kind {
        RemoteNodeInfoFailureKind::Timeout => RemotePreflightFailureKind::Timeout,
        RemoteNodeInfoFailureKind::Auth => RemotePreflightFailureKind::Auth,
        RemoteNodeInfoFailureKind::HttpStatus => RemotePreflightFailureKind::HttpStatus,
        RemoteNodeInfoFailureKind::InvalidMetadata => RemotePreflightFailureKind::InvalidMetadata,
        RemoteNodeInfoFailureKind::Connect => RemotePreflightFailureKind::Connect,
        RemoteNodeInfoFailureKind::Other => RemotePreflightFailureKind::Other,
    }
}
