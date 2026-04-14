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
    message: String,
) -> RemotePreflightFailure {
    let kind = classify_remote_preflight_failure(&message);
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
        message,
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

fn classify_remote_preflight_failure(message: &str) -> RemotePreflightFailureKind {
    if message.contains("timed out") {
        RemotePreflightFailureKind::Timeout
    } else if message.contains("auth failed") {
        RemotePreflightFailureKind::Auth
    } else if message.contains("HTTP ") {
        RemotePreflightFailureKind::HttpStatus
    } else if message.contains("invalid protobuf") {
        RemotePreflightFailureKind::InvalidMetadata
    } else if message.contains("unavailable at") || message.contains("connect") {
        RemotePreflightFailureKind::Connect
    } else {
        RemotePreflightFailureKind::Other
    }
}
