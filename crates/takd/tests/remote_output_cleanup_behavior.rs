use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{ErrorResponse, GetTaskResultResponse, PollTaskEventsResponse};
use takd::{SubmitAttemptStore, build_submit_idempotency_key, handle_remote_v1_request};

#[path = "support/remote_output.rs"]
mod remote_output;
mod support;

use remote_output::{submit_shell_task, test_context};
use support::env::{EnvGuard, env_lock};

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

    let submit_ack = submit_shell_task(
        &context,
        &store,
        "task-run-1",
        "mkdir -p dist && printf 'hello remote\\n' > dist/out.txt",
    );
    assert!(submit_ack.accepted);

    wait_for_terminal_events(&context, &store, "task-run-1");

    let submit_key = build_submit_idempotency_key("task-run-1", Some(1)).expect("submit key");
    let execution_root = exec_root_base.join(submit_key.replace(':', "_"));
    assert!(
        !execution_root.exists(),
        "finished remote execution root should be removed: {}",
        execution_root.display()
    );

    let result =
        handle_remote_v1_request(&context, &store, "GET", "/v1/tasks/task-run-1/result", None)
            .expect("result response");
    let result = GetTaskResultResponse::decode(result.body.as_slice()).expect("decode result");
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0].path, "dist/out.txt");

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

    let missing = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/task-run-1/outputs?path=dist/out.txt",
        None,
    )
    .expect("missing output response");
    assert_eq!(missing.status_code, 404);
    let error = ErrorResponse::decode(missing.body.as_slice()).expect("decode output error");
    assert_eq!(error.message, "output_not_found");
}

fn wait_for_terminal_events(
    context: &takd::RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) {
    let path = format!("/v1/tasks/{task_run_id}/events");
    for _ in 0..50 {
        let events =
            handle_remote_v1_request(context, store, "GET", &path, None).expect("events response");
        let events = PollTaskEventsResponse::decode(events.body.as_slice()).expect("decode events");
        if events.done {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for terminal remote events");
}
