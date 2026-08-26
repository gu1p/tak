use crate::engine::remote_result_fetch::{RemoteFetchFailure, format_remote_fetch_failure};

use super::support::{direct_target, error_body};

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

    assert!(rendered.contains("endpoint: http://node.example"));
    assert!(rendered.contains("daemon_task_handle: handle-7"));
    assert!(rendered.contains("remote_detail: request_failed: database is locked"));
}
