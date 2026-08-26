use crate::engine::protocol_events::remote_protocol_events;

use super::support::*;

#[tokio::test]
async fn events_stream_aborts_on_persistent_result_probe_failure() {
    let (listener, endpoint) = bind_local().await;
    let mut responses = Vec::new();
    for _ in 0..31 {
        responses.push((200, events_body(Vec::new(), false)));
        responses.push((500, error_body("request_failed: database is locked")));
    }
    let server = spawn_http_server(listener, responses);
    let target = direct_target(endpoint);

    let err = remote_protocol_events(&target, "task-run", &task_label(), 1, None)
        .await
        .expect_err("persistent result-probe failures must abort");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("result fetch failed"));
    assert!(rendered.contains("http_status: 500"));
    assert_eq!(server.await.expect("server"), 62);
}
