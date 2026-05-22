#![cfg(test)]

use super::*;

#[path = "queued_failure_tests_support.rs"]
mod support;

use support::{
    TestQueuedSubmit, admit_resources, assert_failed_and_unregistered, poison_status_state,
};

#[test]
fn queued_submit_register_active_job_failure_persists_terminal_failure_and_unregisters() {
    let case = TestQueuedSubmit::new("task-run-queued");
    let execution = case.execution();
    admit_resources(&case.context, &case.idempotency_key, &case.payload);
    poison_status_state(&case.context);

    run_remote_worker_submit_execution(&execution);

    assert_failed_and_unregistered(&case, "node status state lock poisoned", "task-run-queued");
}

#[test]
fn queued_submit_admission_wait_failure_persists_terminal_failure_and_unregisters() {
    let case = TestQueuedSubmit::new("task-run-wait-error");
    let execution = case.execution();
    case.context.poison_resource_admission_for_tests();

    run_remote_worker_submit_execution(&execution);

    assert_failed_and_unregistered(
        &case,
        "resource admission lock poisoned",
        "task-run-wait-error",
    );
}
