use takd::{RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

use crate::support;

use support::remote_output::{submit_shell_task_with_outputs, test_context_with_runtime};
use support::wait_for_terminal_events::wait_for_terminal_events;

#[test]
fn finished_remote_task_serves_outputs_after_execution_root_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let initial_exec_root = temp.path().join("root-a/exec-root");
    let changed_exec_root = temp.path().join("root-b/exec-root");

    let context = test_context_with_runtime(
        RemoteRuntimeConfig::for_tests().with_explicit_remote_exec_root(initial_exec_root.clone()),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit_ack = submit_shell_task_with_outputs(
        &context,
        &store,
        "task-run-root-switch",
        "mkdir -p dist && printf 'hello remote\\n' > dist/out.txt",
        vec![tak_proto::OutputSelector {
            kind: Some(tak_proto::output_selector::Kind::Path(
                "dist/out.txt".to_string(),
            )),
        }],
    );
    assert!(submit_ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-root-switch");
    let changed_context = test_context_with_runtime(
        RemoteRuntimeConfig::for_tests().with_explicit_remote_exec_root(changed_exec_root),
    );

    let output = handle_remote_v1_request(
        &changed_context,
        &store,
        "GET",
        "/v1/tasks/task-run-root-switch/outputs?path=dist/out.txt",
        None,
    )
    .expect("output response");
    assert_eq!(output.status_code, 200);
    assert_eq!(output.body, b"hello remote\n");
}
