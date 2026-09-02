use std::fs;

use tak_exec::execute_remote_worker_steps_with_output_and_cancellation;

use crate::support::{shell_step, worker_spec};

#[tokio::test]
async fn expired_timeout_wins_over_an_immediate_local_completion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let spec = worker_spec("timeout", vec![shell_step("true")], Some(0), None, "local");

    assert_timed_out(temp.path(), &spec).await;
}

async fn assert_timed_out(root: &std::path::Path, spec: &tak_exec::RemoteWorkerExecutionSpec) {
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");
    let outcome = execute_remote_worker_steps_with_output_and_cancellation(
        &workspace,
        spec,
        None,
        &tak_exec::RunCancellation::default(),
    )
    .await
    .expect("timeout should be a terminal task result");

    assert!(!outcome.success);
    assert_eq!(outcome.exit_code, None);
}
