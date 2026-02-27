//! Behavioral tests for remote submit idempotency tuple handling.

use takd::build_submit_idempotency_key;

#[test]
fn key_is_stable_for_identical_task_run_tuple() {
    let first = build_submit_idempotency_key("task-run-123", Some(1)).expect("first key");
    let second = build_submit_idempotency_key("task-run-123", Some(1)).expect("second key");

    assert_eq!(first, "task-run-123:1");
    assert_eq!(first, second);
}

#[test]
fn key_changes_when_attempt_increments() {
    let first_attempt = build_submit_idempotency_key("task-run-123", Some(1)).expect("first");
    let second_attempt = build_submit_idempotency_key("task-run-123", Some(2)).expect("second");

    assert_ne!(first_attempt, second_attempt);
}

#[test]
fn rejects_missing_or_invalid_attempt_before_submit() {
    let missing_attempt =
        build_submit_idempotency_key("task-run-123", None).expect_err("attempt must be required");
    assert!(missing_attempt.to_string().contains("attempt is required"));

    let zero_attempt =
        build_submit_idempotency_key("task-run-123", Some(0)).expect_err("attempt must be >= 1");
    assert!(zero_attempt.to_string().contains("attempt must be >= 1"));
}
