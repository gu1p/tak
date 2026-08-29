use prost::Message;
use tak_proto::{ErrorResponse, ExecutionSession, SubmitTaskRequest};
use takd::SubmitAttemptStore;

use crate::support::{
    remote_output::{empty_workspace_zip, test_container_runtime, test_context_with_runtime},
    runtime_config,
    wait_for_terminal_events::wait_for_terminal_events,
};

#[test]
fn remote_submit_rejects_session_keys_that_escape_the_session_store() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime = runtime_config::builder()
        .with_explicit_remote_exec_root(temp.path().join("exec-root"))
        .with_skip_exec_root_probe(true)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    for (index, key) in [".", ".."].into_iter().enumerate() {
        let task_run_id = format!("reserved-session-key-{index}");
        let submit = SubmitTaskRequest {
            task_run_id: task_run_id.clone(),
            attempt: 1,
            workspace_zip: empty_workspace_zip(),
            runtime: Some(test_container_runtime()),
            task_label: "//:probe".into(),
            session: Some(ExecutionSession {
                key: key.into(),
                name: "probe".into(),
                reuse: "share_workspace".into(),
                share_paths: Vec::new(),
            }),
            ..Default::default()
        };

        let response = takd::daemon::remote::handle_remote_v1_request(
            &context,
            &store,
            "POST",
            "/v1/tasks/submit",
            &[],
            Some(&submit.encode_to_vec()),
        )
        .expect("submit response");
        if response.status_code == 200 {
            wait_for_terminal_events(&context, &store, &task_run_id);
        }

        assert_eq!(response.status_code, 400, "reserved key {key:?}");
        let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
        assert_eq!(error.message, "invalid_submit_fields");
    }

    assert!(store.task_attempt_summaries(false, 10).unwrap().is_empty());
}
