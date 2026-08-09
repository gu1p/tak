#[path = "task_logs_failure_contract/support.rs"]
mod contract_support;

#[test]
fn task_logs_prints_terminal_failure_message_to_stderr() {
    let output = contract_support::run_task_logs_with_terminal_event(
        "task-run-failed",
        r#"{"kind":"TASK_FAILED","timestamp_ms":2,"exit_code":137}"#,
    );

    assert!(output.status.success(), "takd task logs should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "remote task failed with exit code 137\n"
    );
}

#[test]
fn task_logs_prints_terminal_cancelled_exit_code_to_stderr() {
    let output = contract_support::run_task_logs_with_terminal_event(
        "task-run-cancelled",
        r#"{"kind":"TASK_CANCELLED","timestamp_ms":2,"success":false,"exit_code":137}"#,
    );

    assert!(output.status.success(), "takd task logs should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "remote task cancelled with exit code 137\n"
    );
}
