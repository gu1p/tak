#![cfg(test)]
//! Integration coverage for the result-fetch retry wrapper, the events-stream
//! 5xx resume, and the rich failure formatter. Uses the same raw HTTP/1.1
//! `TcpListener` mock pattern as `protocol_result_http_timeout_tests`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use prost::Message;
use tak_core::model::{BackoffDef, TaskLabel};
use tak_proto::{ErrorResponse, GetTaskResultResponse, PollTaskEventsResponse, RemoteEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::protocol_events::remote_protocol_events;
use super::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};
use super::remote_result_fetch::{
    RemoteFetchFailure, ResultFetchPolicy, fetch_remote_result_with_policy,
    format_remote_fetch_failure,
};
use super::{TaskOutputChunk, TaskOutputObserver};

fn direct_target(endpoint: String) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint,
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
        remote_selection: tak_core::model::RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}

fn task_label() -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: "demo".into(),
    }
}

/// A fast policy with no backoff so retry tests complete near-instantly.
fn fast_policy() -> ResultFetchPolicy {
    ResultFetchPolicy {
        max_attempts: 3,
        not_found_grace: 3,
        backoff: BackoffDef::Fixed { seconds: 0.0 },
        not_found_backoff: Duration::ZERO,
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

fn result_body(success: bool) -> Vec<u8> {
    GetTaskResultResponse {
        success,
        status: if success { "success" } else { "failure" }.into(),
        node_id: "builder-a".into(),
        transport_kind: "direct".into(),
        ..GetTaskResultResponse::default()
    }
    .encode_to_vec()
}

fn error_body(message: &str) -> Vec<u8> {
    ErrorResponse {
        message: message.to_string(),
    }
    .encode_to_vec()
}

fn events_body(events: Vec<RemoteEvent>, done: bool) -> Vec<u8> {
    PollTaskEventsResponse { events, done }.encode_to_vec()
}

fn stdout_event(seq: u64, bytes: &[u8]) -> RemoteEvent {
    RemoteEvent {
        seq,
        kind: "TASK_STDOUT_CHUNK".into(),
        chunk_bytes: bytes.to_vec(),
        ..RemoteEvent::default()
    }
}

/// Serves a fixed sequence of `(status, body)` responses, one per accepted
/// connection (the client closes each connection — `Connection: close`). Returns
/// the number of requests actually served, so tests can assert request counts.
fn spawn_http_server(
    listener: TcpListener,
    responses: Vec<(u16, Vec<u8>)>,
) -> tokio::task::JoinHandle<usize> {
    tokio::spawn(async move {
        let mut served = 0_usize;
        for (status, body) in responses {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut buf = vec![0_u8; 2048];
            let _ = stream.read(&mut buf).await;
            let head = format!(
                "HTTP/1.1 {status} {}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                reason_phrase(status),
                body.len()
            );
            if stream.write_all(head.as_bytes()).await.is_err() {
                break;
            }
            if stream.write_all(&body).await.is_err() {
                break;
            }
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
            served += 1;
        }
        served
    })
}

async fn bind_local() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    (listener, format!("http://{addr}"))
}

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
async fn persistent_500_fails_with_rich_message() {
    let (listener, endpoint) = bind_local().await;
    // max_attempts = 3 -> 4 requests before giving up.
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

    assert!(
        rendered.contains("remote node builder-a result fetch failed"),
        "{rendered}"
    );
    assert!(rendered.contains("transport: direct"), "{rendered}");
    assert!(rendered.contains("task_run_id: task-run"), "{rendered}");
    assert!(rendered.contains("attempt: 2"), "{rendered}");
    assert!(
        rendered.contains("path: /v1/tasks/task-run/result"),
        "{rendered}"
    );
    assert!(rendered.contains("http_status: 500"), "{rendered}");
    assert!(
        rendered.contains("daemon_task_handle: handle-xyz"),
        "{rendered}"
    );
    assert!(rendered.contains("database is locked"), "{rendered}");
    assert_eq!(server.await.expect("server"), 4);
}

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

    assert!(rendered.contains("http_status: 400"), "{rendered}");
    assert!(rendered.contains("bad_request"), "{rendered}");
    // Exactly one request — no retry on a non-retryable status.
    assert_eq!(server.await.expect("server"), 1);
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

#[tokio::test]
async fn persistent_404_reports_missing_result() {
    let (listener, endpoint) = bind_local().await;
    // not_found_grace = 3 -> 4 requests before declaring missing.
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

    assert!(rendered.contains("http_status: 404"), "{rendered}");
    assert!(rendered.contains("result still missing"), "{rendered}");
    assert_eq!(server.await.expect("server"), 4);
}

#[derive(Default)]
struct CapturingObserver {
    output: Mutex<Vec<u8>>,
}

impl TaskOutputObserver for CapturingObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        self.output
            .lock()
            .expect("output lock")
            .extend_from_slice(&chunk.bytes);
        Ok(())
    }
}

#[tokio::test]
async fn events_stream_resumes_after_5xx_without_duplicate_output() {
    let (listener, endpoint) = bind_local().await;
    let server = spawn_http_server(
        listener,
        vec![
            // First poll (after_seq=0): seq 1..3, not done.
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
            // Second poll (after_seq=3): a transient 500.
            (500, error_body("request_failed: database is locked")),
            // Resume poll (after_seq=3): re-sends seq 3 plus seq 4, done.
            // The parser must drop seq 3 (already seen) and emit only seq 4.
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
            .expect("events stream should survive a transient 5xx and finish");

    assert!(result.is_none(), "terminal event yields no inline result");
    let captured = observer.output.lock().expect("output lock").clone();
    assert_eq!(captured, b"abcd", "each chunk must appear exactly once");
    let seqs: Vec<u64> = logs.iter().map(|chunk| chunk.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3, 4]);
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

    assert!(rendered.contains("events fetch failed"), "{rendered}");
    assert!(rendered.contains("http_status: 400"), "{rendered}");
    assert_eq!(server.await.expect("server"), 2);
}

#[tokio::test]
async fn events_stream_aborts_on_persistent_result_probe_failure() {
    // The events endpoint stays healthy (200, empty, not done) so the task looks
    // alive, but the result endpoint persistently 500s. The bounded probe-failure
    // budget (MAX_EVENT_RECONNECTS = 30 internally) must eventually abort with the
    // rich error instead of polling forever. 31 quiet iterations each issue one
    // events poll then one result probe -> 62 requests, aborting on the last.
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
        .expect_err("a persistent result-probe 500 must eventually abort");
    let rendered = format!("{err:#}");

    assert!(rendered.contains("result fetch failed"), "{rendered}");
    assert!(rendered.contains("http_status: 500"), "{rendered}");
    assert_eq!(server.await.expect("server"), 62);
}

#[test]
fn formatter_includes_handle_and_decoded_detail() {
    let mut target = direct_target("http://node.example".into());
    target.daemon_task_handle = Some("handle-7".into());
    let body = error_body("request_failed: database is locked");

    let rendered = format_remote_fetch_failure(&RemoteFetchFailure {
        target: &target,
        task_run_id: "tr-1",
        attempt: 3,
        phase: "result",
        path: "/v1/tasks/tr-1/result",
        status: Some(500),
        body: Some(&body),
        transport_error: None,
    });

    assert!(
        rendered.contains("endpoint: http://node.example"),
        "{rendered}"
    );
    assert!(
        rendered.contains("daemon_task_handle: handle-7"),
        "{rendered}"
    );
    assert!(
        rendered.contains("remote_detail: request_failed: database is locked"),
        "{rendered}"
    );
}
