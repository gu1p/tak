use prost::Message;
use tak_proto::{ListTaskAttemptsResponse, SubmitTaskResponse};
use takd::SubmitAttemptStore;

#[path = "remote_task_list_contract/registration_key.rs"]
mod registration_key;

use registration_key::SubmitRegistrationKey;

#[test]
fn remote_tasks_route_lists_persisted_task_attempts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let context = crate::support::remote_output::test_context();
    let execution_root = temp.path().join("exec");
    let key = store
        .register_submit_with_execution_root_base(
            "task-run-list",
            Some(2),
            "//apps/web:build",
            None,
            "builder-a",
            &execution_root,
        )
        .expect("register submit")
        .idempotency_key()
        .to_string();
    store
        .set_result_payload(&key, r#"{"success":true}"#)
        .expect("complete submit");

    let response = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks?state=all",
        &[],
        None,
    )
    .expect("task list response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, "application/x-protobuf");
    let tasks =
        ListTaskAttemptsResponse::decode(response.body.as_slice()).expect("decode task list");
    assert_eq!(tasks.attempts.len(), 1);
    let task = &tasks.attempts[0];
    assert_eq!(task.task_run_id, "task-run-list");
    assert_eq!(task.attempt, 2);
    assert_eq!(task.task_label, "//apps/web:build");
    assert_eq!(task.node_id, "builder-a");
    assert_eq!(task.state, "completed");
}

#[test]
fn remote_tasks_route_lists_execution_labels_when_submitted() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime_config = crate::support::runtime_config::builder()
        .with_skip_exec_root_probe(true)
        .build();
    let context = crate::support::remote_output::test_context_with_runtime(runtime_config);
    let mut submit =
        crate::support::remote_v1_http_submit::submit_request("task-run-lineage", Vec::new());
    submit.execution_label = Some("check.build".to_string());
    let response = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        &[],
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    let ack = SubmitTaskResponse::decode(response.body.as_slice()).expect("decode submit");
    assert!(ack.accepted);

    let response = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks?state=all",
        &[],
        None,
    )
    .expect("task list response");

    let tasks =
        ListTaskAttemptsResponse::decode(response.body.as_slice()).expect("decode task list");
    assert_eq!(tasks.attempts.len(), 1);
    assert_eq!(
        tasks.attempts[0].execution_label.as_deref(),
        Some("check.build")
    );
}
