use prost::Message;
use tak_proto::ListTaskAttemptsResponse;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_tasks_route_lists_persisted_task_attempts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let context = crate::support::remote_output::test_context();
    let execution_root = temp.path().join("exec");
    let key = store
        .register_submit_with_task_label(
            "task-run-list",
            Some(2),
            "//apps/web:build",
            "builder-a",
            &execution_root,
        )
        .expect("register submit")
        .idempotency_key()
        .to_string();
    store
        .set_result_payload(&key, r#"{"success":true}"#)
        .expect("complete submit");

    let response = handle_remote_v1_request(&context, &store, "GET", "/v1/tasks?state=all", None)
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

trait SubmitRegistrationKey {
    fn idempotency_key(&self) -> &str;
}

impl SubmitRegistrationKey for takd::SubmitRegistration {
    fn idempotency_key(&self) -> &str {
        match self {
            Self::Created { idempotency_key } | Self::Attached { idempotency_key } => {
                idempotency_key
            }
        }
    }
}
