#![cfg(test)]

use crate::engine::preflight_failure::{
    RemoteNodeInfoFailureKind, classify_preflight_failure_kind,
};
use crate::engine::remote_diagnostics::RemotePreflightFailureKind;

#[test]
fn typed_node_info_failures_map_to_preflight_failure_kinds() {
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::Timeout),
        RemotePreflightFailureKind::Timeout
    );
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::Auth),
        RemotePreflightFailureKind::Auth
    );
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::HttpStatus),
        RemotePreflightFailureKind::HttpStatus
    );
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::InvalidMetadata),
        RemotePreflightFailureKind::InvalidMetadata
    );
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::Connect),
        RemotePreflightFailureKind::Connect
    );
    assert_eq!(
        classify_preflight_failure_kind(RemoteNodeInfoFailureKind::Other),
        RemotePreflightFailureKind::Other
    );
}
