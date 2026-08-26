use std::sync::Arc;

use crate::engine::TaskOutputObserver;
use crate::engine::protocol_events::remote_protocol_events;

use super::support::*;

#[tokio::test]
async fn events_stream_resumes_after_5xx_without_duplicate_output() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        vec![
            (
                200,
                events_body(
                    vec![
                        stdout_event(1, b"a"),
                        stdout_event(2, b"b"),
                        stdout_event(3, b"c"),
                    ],
                    false,
                ),
            ),
            (500, error_body("request_failed: database is locked")),
            (
                200,
                events_body(vec![stdout_event(3, b"c"), stdout_event(4, b"d")], true),
            ),
        ],
    );
    let target = direct_target(endpoint);
    let observer = Arc::new(CapturingObserver::default());
    let dyn_observer: Arc<dyn TaskOutputObserver> = observer.clone();

    let (logs, result) =
        remote_protocol_events(&target, "task-run", &task_label(), 1, Some(&dyn_observer))
            .await
            .expect("events stream should resume");

    assert!(result.is_none());
    assert_eq!(*observer.output.lock().expect("output lock"), b"abcd");
    assert_eq!(
        logs.iter().map(|chunk| chunk.seq).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(server.await.expect("server"), 3);
}

#[tokio::test]
async fn events_stream_4xx_is_fatal() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        vec![
            (
                200,
                events_body(vec![stdout_event(1, b"a"), stdout_event(2, b"b")], false),
            ),
            (400, error_body("bad_request")),
        ],
    );
    let target = direct_target(endpoint);

    let err = remote_protocol_events(&target, "task-run", &task_label(), 1, None)
        .await
        .expect_err("a 4xx on the events stream is fatal");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("events fetch failed"));
    assert!(rendered.contains("http_status: 400"));
    assert_eq!(server.await.expect("server"), 2);
}
