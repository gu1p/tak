use crate::engine::remote_result_fetch::fetch_remote_result_with_policy;

use super::support::*;

#[tokio::test]
async fn ordinary_4xx_fails_fast() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(listener, vec![(400, error_body("bad_request"))]);
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
    .expect_err("4xx should fail fast");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("http_status: 400"));
    assert!(rendered.contains("bad_request"));
    assert_eq!(server.await.expect("server"), 1);
}
