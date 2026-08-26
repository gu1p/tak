use tak_proto::RemoteFailureKind as WireFailureKind;

use super::remote_failure::{RemoteFailureKind, classify_remote_result_failure};

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
}

#[test]
fn legacy_exit_137_is_infrastructure_but_other_exits_are_task_failures() {
    assert_eq!(
        classify_remote_result_failure(false, Some(137), None),
        Some(RemoteFailureKind::Infrastructure)
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
