use std::time::Duration;

use takd::{RemoteNodeContext, SubmitAttemptStore, build_submit_idempotency_key};

use super::remote_session::cancel_session_task;

pub async fn wait_for_session_task_inactive(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    while cancel_session_task(context, store, task_run_id) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for session task to become inactive"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

pub fn assert_cancelled_result(store: &SubmitAttemptStore, task_run_id: &str) {
    let key = build_submit_idempotency_key(task_run_id, Some(1)).expect("build submit key");
    let result = store
        .result_payload(&key)
        .expect("query terminal result")
        .expect("persist terminal result");
    assert!(result.contains(r#""status":"cancelled""#), "{result}");
}
