#![cfg(test)]

use super::super::super::super::{
    RemoteHttpExchangeErrorKind, StrictRemoteTarget, remote_models::StrictRemoteTransportKind,
};

#[test]
fn bad_gateway_broker_error_reports_remote_unavailable() {
    let err = super::broker_error_response(
        &target(),
        b"connect_failed: connection refused",
        "connect_failed",
        502,
    );

    assert_eq!(err.kind, RemoteHttpExchangeErrorKind::Connect);
    assert!(err.message.contains("remote node builder-a unavailable"));
    assert!(!err.message.contains("broker unavailable"));
}

#[test]
fn bad_request_broker_error_is_not_a_connect_failure() {
    let err = super::broker_error_response(
        &target(),
        b"unsupported_remote_transport",
        "unsupported_remote_transport",
        400,
    );

    assert_eq!(err.kind, RemoteHttpExchangeErrorKind::Other);
    assert!(
        err.message
            .contains("local takd Tor broker rejected request")
    );
}

fn target() -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://builder-a.onion".into(),
        transport_kind: StrictRemoteTransportKind::Tor,
        bearer_token: "secret".into(),
        runtime: None,
        remote_selection: tak_core::model::RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}
