use anyhow::anyhow;
use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};

use super::daemon_error;
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
        anyhow!(
            "No known remote worker satisfies this task's requirements.\n\nTask requires:\n  cpu: 16.00\n\nLargest known worker:\n  cpu: 8.00"
        ),
    );

    assert!(!err.message.contains("__takd_daemon_tor__"));
    assert!(!err.message.contains("remote node __takd_daemon_tor__"));
    assert!(err.message.contains("subsystem: placement"));
    assert!(err.message.contains("stage: remote placement"));
    assert!(err.message.contains("retryable: no"));
    assert!(
        err.message
            .contains("No known remote worker satisfies this task's requirements")
    );
    assert!(err.message.contains("source:"));
}
