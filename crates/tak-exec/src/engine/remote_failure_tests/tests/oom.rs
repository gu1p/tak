use tak_proto::RemoteFailureKind as WireFailureKind;

// Container OOM classification requires explicit wire evidence.

use super::super::remote_failure::{RemoteFailureKind, classify_remote_result_failure};

#[test]
fn confirmed_container_oom_is_infrastructure_failure() {
    assert_eq!(
        classify_remote_result_failure(
            false,
            Some(137),
            Some(WireFailureKind::ContainerOom as i32),
        ),
        Some(RemoteFailureKind::Infrastructure)
    );
}

#[test]
fn legacy_infrastructure_exit_137_is_not_evidence_of_infrastructure_failure() {
    assert_eq!(
        classify_remote_result_failure(
            false,
            Some(137),
            Some(WireFailureKind::Infrastructure as i32),
        ),
        Some(RemoteFailureKind::Unknown)
    );
}
