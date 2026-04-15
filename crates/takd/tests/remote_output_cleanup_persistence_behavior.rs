use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{SubmitAttemptStore, build_submit_idempotency_key, handle_remote_v1_request};

#[path = "support/remote_output.rs"]
mod remote_output;
mod support;
#[path = "support/wait_for_terminal_events.rs"]
mod wait_for_terminal_events;

use remote_output::{submit_shell_task_with_outputs, test_context};
use support::env::{EnvGuard, env_lock};
use wait_for_terminal_events::wait_for_terminal_events;

#[test]
fn finished_remote_task_serves_outputs_after_execution_root_cleanup() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("exec-root");
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        exec_root_base.display().to_string(),
    );

    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit_ack = submit_shell_task_with_outputs(
        &context,
        &store,
        "task-run-1",
        "mkdir -p dist && printf 'hello remote\\n' > dist/out.txt",
        vec![tak_proto::OutputSelector {
            kind: Some(tak_proto::output_selector::Kind::Path(
                "dist/out.txt".to_string(),
            )),
        }],
    );
    assert!(submit_ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-1");

    let submit_key = build_submit_idempotency_key("task-run-1", Some(1)).expect("submit key");
    let execution_root = exec_root_base.join(submit_key.replace(':', "_"));
    assert!(!execution_root.exists(), "execution root should be removed");

    let result =
        handle_remote_v1_request(&context, &store, "GET", "/v1/tasks/task-run-1/result", None)
            .expect("result response");
    let result = GetTaskResultResponse::decode(result.body.as_slice()).expect("decode result");
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0].path, "dist/out.txt");

    for _ in 0..3 {
        let output = handle_remote_v1_request(
            &context,
            &store,
            "GET",
            "/v1/tasks/task-run-1/outputs?path=dist/out.txt",
            None,
        )
        .expect("output response");
        assert_eq!(output.status_code, 200);
        assert_eq!(output.body, b"hello remote\n");
    }
}
