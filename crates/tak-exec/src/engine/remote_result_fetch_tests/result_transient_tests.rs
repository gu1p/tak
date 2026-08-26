use crate::engine::remote_result_fetch::fetch_remote_result_with_policy;

use super::support::*;

#[tokio::test]
async fn transient_500_then_200_succeeds() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        vec![
            (500, error_body("request_failed: database is locked")),
            (200, result_body(true)),
        ],
    );
    let target = direct_target(endpoint);

    let result = fetch_remote_result_with_policy(
        &target,
        "task-run",
        1,
        &task_label(),
        None,
        &fast_policy(),
    )
    .await
    .expect("transient 500 should be retried into success");

    assert!(result.success);
    assert_eq!(server.await.expect("server"), 2);
}

#[tokio::test]
async fn not_found_grace_then_200_succeeds() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        vec![
            (404, error_body("result_not_found")),
            (404, error_body("result_not_found")),
            (200, result_body(true)),
        ],
    );
    let target = direct_target(endpoint);

    let result = fetch_remote_result_with_policy(
        &target,
        "task-run",
        1,
        &task_label(),
        None,
        &fast_policy(),
    )
    .await
    .expect("404 within grace should be retried into success");

    assert!(result.success);
    assert_eq!(server.await.expect("server"), 3);
}
