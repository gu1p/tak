use prost::Message;
use tak_proto::{ErrorResponse, NodeStatusResponse};

use crate::support::remote_v1_http::{decode_error_response, send_raw_request, start_server};
use crate::support::remote_v1_http_submit::truncated_submit_request;
use takd::handle_remote_v1_request;

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
    let status = handle_remote_v1_request(
        &server.context,
        &server.store,
        "GET",
        "/v1/node/status",
        None,
    )
    .expect("status response");
    let status = NodeStatusResponse::decode(status.body.as_slice()).expect("decode node status");
    assert!(status.active_jobs.is_empty());
    let events = handle_remote_v1_request(
        &server.context,
        &server.store,
        "GET",
        &format!("/v1/tasks/{task_run_id}/events"),
        None,
    )
    .expect("events response");
    assert_eq!(events.status_code, 404);
    let error = ErrorResponse::decode(events.body.as_slice()).expect("decode not found error");
    assert_eq!(error.message, "task_not_found");
}
