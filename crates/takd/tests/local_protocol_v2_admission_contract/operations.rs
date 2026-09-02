use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test(flavor = "multi_thread")]
async fn all_v2_run_operations_are_recognized_on_one_persistent_connection() {
    let mut daemon = RawLocalProtocol::start().await;
    let list = daemon
        .exchange(r#"{"protocol_version":2,"request_id":"list","operation":{"type":"ListRuns"}}"#)
        .await;
    super::assert_json_response(
        &list,
        serde_json::json!({
            "protocol_version": 2,
            "type": "RunList",
            "request_id": "list",
            "runs": []
        }),
    );
    let status = daemon
        .exchange(r#"{"protocol_version":2,"request_id":"status","operation":{"type":"GetDaemonStatus"}}"#)
        .await;
    super::assert_json_response(
        &status,
        serde_json::json!({
            "protocol_version": 2,
            "type": "DaemonStatus",
            "request_id": "status",
            "status": {
                "active_leases": 0,
                "pending_requests": 0,
                "limiter_count": 0
            }
        }),
    );
    let requests = [
        ("show", r#"{"type":"GetRun","run_id":"run-123"}"#),
        (
            "attach",
            r#"{"type":"AttachRun","run_id":"run-123","after_event":0}"#,
        ),
        ("cancel", r#"{"type":"CancelRun","run_id":"run-123"}"#),
        (
            "outputs",
            r#"{"type":"GetOutputManifest","run_id":"run-123"}"#,
        ),
    ];

    for (request_id, operation) in requests {
        let request = format!(
            r#"{{"protocol_version":2,"request_id":"{request_id}","operation":{operation}}}"#
        );
        let response = daemon.exchange(&request).await;
        super::assert_json_response(
            &response,
            serde_json::json!({
                "protocol_version": 2,
                "type": "Error",
                "request_id": request_id,
                "message": "The requested run does not exist.",
                "code": "run_not_found",
                "retryable": false
            }),
        );
    }

    let legacy = daemon
        .exchange(r#"{"type":"Status","request_id":"legacy-status"}"#)
        .await;
    assert!(legacy.contains(r#""code":"protocol_version_unsupported""#));
    assert!(legacy.contains("Upgrade tak, takd, and workers together"));
}
