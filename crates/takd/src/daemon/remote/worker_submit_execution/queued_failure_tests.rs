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

// Regression for the orphan-watchdog flake: when the run is cancelled (the
// shared latch is tripped, e.g. by the watchdog) but the container still raced
// to a non-zero exit in the same window, the terminal status must be cancelled,
// not failure.
#[test]
fn cancelled_run_records_cancelled_even_when_container_exited_nonzero() {
    let case = TestQueuedSubmit::new("task-run-cancel-race");
    let execution = case.execution();
    case.cancellation.cancel();
    let observer = std::sync::Arc::new(RemoteWorkerEventObserver::new_with_next_seq(
        case.store.clone(),
        case.idempotency_key.clone(),
        2,
    ));

    persist_worker_execution_result(
        WorkerExecutionResultPersistence {
            execution: &execution,
            output_observer: observer,
            idempotency_key: &case.idempotency_key,
            started_at: 1,
            finished_at: 2,
            duration_ms: 1,
        },
        Ok((
            tak_runner::RemoteWorkerExecutionResult {
                success: false,
                exit_code: Some(1),
                runtime_kind: None,
                runtime_engine: None,
            },
            Vec::new(),
        )),
    );

    let result = case
        .store
        .result_payload(&case.idempotency_key)
        .expect("result query")
        .expect("terminal result");
    assert!(
        result.contains(r#""status":"cancelled""#),
        "cancelled run must record cancelled, got: {result}"
    );
}
