use crate::support;

use std::fs;

use prost::Message;
use tak_proto::ErrorResponse;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_node_logs_route_serves_complete_service_log() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(
        state_root.join("service.log"),
        "booting takd\nremote service ready\n",
    )
    .expect("write service log");
    let context = support::remote_output::test_context().with_state_root(&state_root);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let response =
        handle_remote_v1_request(&context, &store, "GET", "/v1/node/logs?all=true", None)
            .expect("logs response");

    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, "text/plain; charset=utf-8");
    assert_eq!(
        String::from_utf8_lossy(&response.body),
        "booting takd\nremote service ready\n"
    );
}

#[test]
fn remote_node_logs_route_tails_requested_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(state_root.join("service.log"), "line-1\nline-2\nline-3\n")
        .expect("write service log");
    let context = support::remote_output::test_context().with_state_root(&state_root);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let response = handle_remote_v1_request(&context, &store, "GET", "/v1/node/logs?lines=2", None)
        .expect("logs response");

    assert_eq!(response.status_code, 200);
    assert_eq!(String::from_utf8_lossy(&response.body), "line-2\nline-3\n");
}

#[test]
fn remote_node_logs_route_reports_missing_service_log() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let context = support::remote_output::test_context().with_state_root(&state_root);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let response = handle_remote_v1_request(&context, &store, "GET", "/v1/node/logs", None)
        .expect("logs response");

    assert_eq!(response.status_code, 404);
    let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error response");
    assert_eq!(error.message, "service_log_not_found");
}
