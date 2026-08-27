use tak_proto::RemoteFailureKind as WireFailureKind;

use super::remote_failure::{RemoteFailureKind, classify_remote_result_failure};

#[path = "remote_failure_tests/tests/oom.rs"]
mod oom;

#[test]
fn explicit_wire_failure_kinds_are_authoritative() {
    assert_eq!(
        classify_remote_result_failure(
            false,
            Some(1),
            Some(WireFailureKind::Infrastructure as i32)
        ),
        Some(RemoteFailureKind::Infrastructure)
    );
    assert_eq!(
        classify_remote_result_failure(false, None, Some(WireFailureKind::Cancellation as i32)),
        Some(RemoteFailureKind::Cancellation)
    );
    assert_eq!(
        classify_remote_result_failure(
            false,
            Some(137),
            Some(WireFailureKind::ResourceCapacity as i32)
        ),
        Some(RemoteFailureKind::ResourceCapacity)
    );
}

#[test]
fn unattributed_exit_137_is_unknown_and_does_not_trigger_infrastructure_failover() {
    assert_eq!(
        classify_remote_result_failure(false, Some(137), None),
        Some(RemoteFailureKind::Unknown)
    );
    assert_eq!(
        classify_remote_result_failure(false, Some(1), None),
        Some(RemoteFailureKind::Task)
    );
    assert_eq!(classify_remote_result_failure(true, Some(0), None), None);
}

#[test]
fn cancellation_never_enters_the_authored_retry_policy() {
    assert!(!super::remote_failure::permits_authored_retry(Some(
        RemoteFailureKind::Cancellation,
    )));
    assert!(super::remote_failure::permits_authored_retry(Some(
        RemoteFailureKind::Task,
    )));
}

#[test]
fn only_evidenced_infrastructure_and_capacity_failures_change_workers() {
    assert!(super::remote_failure::requires_worker_failover(Some(
        RemoteFailureKind::Infrastructure,
    )));
    assert!(super::remote_failure::requires_worker_failover(Some(
        RemoteFailureKind::ResourceCapacity,
    )));
    assert!(!super::remote_failure::requires_worker_failover(Some(
        RemoteFailureKind::Unknown,
    )));
}

#[test]
fn retry_headline_preserves_typed_resource_capacity() {
    assert_eq!(
        super::remote_failover::retry_failure_message(
            Some(RemoteFailureKind::ResourceCapacity),
            "builder-a",
        ),
        "remote resource-capacity stop on builder-a; retrying on another eligible worker"
    );
    assert_eq!(
        super::remote_failover::retry_failure_message(
            Some(RemoteFailureKind::Infrastructure),
            "builder-a",
        ),
        "remote infrastructure failure on builder-a; retrying on another eligible worker"
    );
}
