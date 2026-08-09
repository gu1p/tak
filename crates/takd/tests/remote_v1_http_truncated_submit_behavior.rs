use prost::Message;
use tak_proto::{ErrorResponse, NodeStatusResponse};

use crate::support::remote_v1_http::{decode_error_response, send_raw_request, start_server};
use crate::support::remote_v1_http_submit::truncated_submit_request;

#[tokio::test]
async fn truncated_submit_body_returns_explicit_bad_request_reason() {
    let server = start_server().await;
    let response = send_raw_request(
        server.addr,
        &truncated_submit_request("task-run-truncated-1"),
    )
    .await;
    assert!(response.head.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert_eq!(decode_error_response(&response).message, "truncated_body");
}

#[tokio::test]
async fn truncated_submit_body_does_not_create_active_job_or_store_state() {
    let server = start_server().await;
    let task_run_id = "task-run-truncated-2";
    let response = send_raw_request(server.addr, &truncated_submit_request(task_run_id)).await;
    assert_eq!(decode_error_response(&response).message, "truncated_body");
    let status = send_raw_request(
        server.addr,
        b"GET /v1/node/status HTTP/1.1\r\nHost: builder\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(status.head.starts_with("HTTP/1.1 200 OK\r\n"));
    let status = NodeStatusResponse::decode(status.body.as_slice()).expect("decode node status");
    assert!(status.active_jobs.is_empty());
    let events_request = format!(
        "GET /v1/tasks/{task_run_id}/events HTTP/1.1\r\nHost: builder\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n"
    );
    let events = send_raw_request(server.addr, events_request.as_bytes()).await;
    assert!(events.head.starts_with("HTTP/1.1 404 Not Found\r\n"));
    let error = ErrorResponse::decode(events.body.as_slice()).expect("decode not found error");
    assert_eq!(error.message, "task_not_found");
}
