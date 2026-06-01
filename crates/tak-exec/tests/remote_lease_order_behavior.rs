#![allow(clippy::await_holding_lock)]

use tak_exec::run_tasks;

use crate::support::{
    RecordingLeaseConfig, fused_remote_cascade_spec, remote_lease_case,
    remote_lease_case_with_slow_result, remote_lease_case_with_submit_failure,
};

#[tokio::test]
async fn remote_task_acquires_lease_before_submit() {
    let case = remote_lease_case("normal").await;

    run_tasks(&case.spec, std::slice::from_ref(&case.label), &case.options)
        .await
        .expect("remote task should run");

    assert_eq!(
        case.events.snapshot(),
        vec!["lease_acquire:ui_lock", "remote_submit", "lease_release"]
    );
}

#[tokio::test]
async fn remote_fused_cascade_acquires_merged_lease_before_submit() {
    let mut case = remote_lease_case("fused").await;
    let label = fused_remote_cascade_spec(&mut case.spec);

    run_tasks(&case.spec, std::slice::from_ref(&label), &case.options)
        .await
        .expect("remote fused cascade should run");

    assert_eq!(
        case.events.snapshot(),
        vec!["lease_acquire:ui_lock", "remote_submit", "lease_release"]
    );
}

#[tokio::test]
async fn remote_submit_failure_releases_acquired_lease() {
    let case = remote_lease_case_with_submit_failure("submit-fails").await;

    let error = run_tasks(&case.spec, std::slice::from_ref(&case.label), &case.options)
        .await
        .expect_err("remote submit should fail");

    assert!(
        error.to_string().contains("submit failed"),
        "error: {error:#}"
    );
    assert_eq!(
        case.events.snapshot(),
        vec!["lease_acquire:ui_lock", "remote_submit", "lease_release"]
    );
}

#[tokio::test]
async fn remote_task_renews_lease_while_running() {
    let case = remote_lease_case_with_slow_result(
        "renew",
        RecordingLeaseConfig {
            ttl_ms: 90,
            renew_after_ms: 10,
        },
        std::time::Duration::from_millis(60),
    )
    .await;

    run_tasks(&case.spec, std::slice::from_ref(&case.label), &case.options)
        .await
        .expect("remote task should run");

    let events = case.events.snapshot();
    assert!(
        events.iter().any(|event| event == "lease_renew"),
        "expected renewal before release, events: {events:?}"
    );
    assert_eq!(
        events.first().map(String::as_str),
        Some("lease_acquire:ui_lock")
    );
    assert_eq!(events.last().map(String::as_str), Some("lease_release"));
}

#[tokio::test]
async fn remote_task_without_needs_does_not_request_lease() {
    let mut case = remote_lease_case("no-needs").await;
    case.spec
        .tasks
        .get_mut(&case.label)
        .expect("task")
        .needs
        .clear();

    run_tasks(&case.spec, std::slice::from_ref(&case.label), &case.options)
        .await
        .expect("remote task should run");

    assert_eq!(case.events.snapshot(), vec!["remote_submit"]);
}
