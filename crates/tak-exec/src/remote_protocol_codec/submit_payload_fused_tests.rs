use tak_core::model::{BackoffDef, ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RetryDef};

use super::submit_payload_test_support::{
    direct_target, encoded_workspace, task_with_steps_and_needs, workspace,
};
use super::*;

#[test]
fn build_remote_submit_payload_includes_fused_member_policies() {
    let target = direct_target(Some(RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "ghcr.io/acme/web:latest".into(),
        },
    }));
    let mut member = task_with_steps_and_needs();
    member.label.name = "flaky".into();
    member.timeout_s = Some(5);
    member.retry = RetryDef {
        attempts: 3,
        on_exit: vec![42],
        backoff: BackoffDef::Fixed { seconds: 0.25 },
    };

    let payload = build_remote_submit_payload(
        &target,
        "task-run-1",
        1,
        &task_with_steps_and_needs(),
        &workspace(&encoded_workspace()),
        None,
        Some(&[member]),
    )
    .expect("submit payload");

    assert_eq!(payload.fused_members.len(), 1);
    let fused = &payload.fused_members[0];
    assert_eq!(fused.task_label, "apps/web:flaky");
    assert_eq!(fused.timeout_s, Some(5));
    assert_eq!(fused.steps.len(), 2);
    let retry = fused.retry.as_ref().expect("retry");
    assert_eq!(retry.attempts, 3);
    assert_eq!(retry.on_exit, vec![42]);
    match retry.backoff.as_ref().expect("backoff").kind.as_ref() {
        Some(tak_proto::retry_backoff::Kind::Fixed(fixed)) => assert_eq!(fixed.seconds, 0.25),
        other => panic!("expected fixed backoff, got {other:?}"),
    }
}
