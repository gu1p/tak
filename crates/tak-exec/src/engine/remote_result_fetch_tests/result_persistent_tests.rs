use crate::engine::remote_result_fetch::fetch_remote_result_with_policy;

use super::support::*;

#[tokio::test]
async fn persistent_500_fails_with_rich_message() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        (0..4)
            .map(|_| (500, error_body("request_failed: database is locked")))
            .collect(),
    );
    let mut target = direct_target(endpoint);
    target.daemon_task_handle = Some("handle-xyz".into());

    let err = fetch_remote_result_with_policy(
        &target,
        "task-run",
        2,
        &task_label(),
        None,
        &fast_policy(),
    )
    .await
    .expect_err("persistent 500 should fail");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("remote node builder-a result fetch failed"));
    assert!(rendered.contains("transport: direct"));
    assert!(rendered.contains("task_run_id: task-run"));
    assert!(rendered.contains("attempt: 2"));
    assert!(rendered.contains("path: /v1/tasks/task-run/result"));
    assert!(rendered.contains("http_status: 500"));
    assert!(rendered.contains("daemon_task_handle: handle-xyz"));
    assert!(rendered.contains("database is locked"));
    assert_eq!(server.await.expect("server"), 4);
}

#[tokio::test]
async fn persistent_404_reports_missing_result() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        (0..4)
            .map(|_| (404, error_body("result_not_found")))
            .collect(),
    );
    let target = direct_target(endpoint);

    let err = fetch_remote_result_with_policy(
        &target,
        "task-run",
        1,
        &task_label(),
        None,
        &fast_policy(),
    )
    .await
    .expect_err("persistent 404 should fail");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("http_status: 404"));
    assert!(rendered.contains("result still missing"));
    assert_eq!(server.await.expect("server"), 4);
}
