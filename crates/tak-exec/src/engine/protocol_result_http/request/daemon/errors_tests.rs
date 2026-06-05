use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};

use super::{DaemonLocalError, daemon_error};
use crate::engine::StrictRemoteTarget;

#[test]
fn daemon_error_for_placeholder_target_reports_local_placement_not_remote_node() {
    let target = StrictRemoteTarget::daemon_tor_placement(&RemoteSpec {
        pool: Some("build".into()),
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Tor,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    });
    let err = daemon_error(
        &target,
        DaemonLocalError::response(
            "No known remote worker satisfies this task's requirements.\n\nTask requires:\n  cpu: 16.00\n\nLargest known worker:\n  cpu: 8.00".into(),
            Some("resource_requirements_exceed_worker_capacity".into()),
            Some(false),
        )
        .into(),
    );

    assert!(!err.message.contains("__takd_daemon_tor__"));
    assert!(!err.message.contains("remote node __takd_daemon_tor__"));
    assert!(err.message.contains("subsystem: placement"));
    assert!(err.message.contains("stage: remote placement"));
    assert!(err.message.contains("retryable: no"));
    assert!(
        err.message
            .contains("code: resource_requirements_exceed_worker_capacity")
    );
    assert!(!err.is_retryable());
    assert!(
        err.message
            .contains("No known remote worker satisfies this task's requirements")
    );
    assert!(err.message.contains("source:"));
}

#[test]
fn daemon_placement_retryability_comes_from_structured_metadata_not_message_text() {
    let target = StrictRemoteTarget::daemon_tor_placement(&RemoteSpec {
        pool: Some("build".into()),
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Tor,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    });
    let err = daemon_error(
        &target,
        DaemonLocalError::response(
            "No known remote worker satisfies this task's requirements.".into(),
            Some("all_tor_peers_unreachable".into()),
            Some(true),
        )
        .into(),
    );

    assert!(err.message.contains("retryable: yes"));
    assert!(err.message.contains("code: all_tor_peers_unreachable"));
    assert!(err.is_retryable());
}

#[test]
fn daemon_placement_errors_without_retry_metadata_fail_closed_with_upgrade_guidance() {
    let target = StrictRemoteTarget::daemon_tor_placement(&RemoteSpec {
        pool: Some("build".into()),
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Tor,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    });
    let err = daemon_error(
        &target,
        DaemonLocalError::response("older daemon response".into(), None, None).into(),
    );

    assert!(err.message.contains("retryable: no"));
    assert!(err.message.contains("restart/update local takd"));
    assert!(!err.is_retryable());
}
