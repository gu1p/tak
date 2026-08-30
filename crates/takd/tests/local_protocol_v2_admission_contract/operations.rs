use crate::support::raw_local_protocol::RawLocalProtocol;

const INACTIVE_MESSAGE: &str = "Protocol v2 run operations are not active in this takd build. Upgrade tak, takd, and workers together.";

#[tokio::test(flavor = "multi_thread")]
async fn all_v2_run_operations_are_recognized_on_one_persistent_connection() {
    let mut daemon = RawLocalProtocol::start().await;
    let requests = [
        ("list", r#"{"type":"ListRuns"}"#),
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
                "message": INACTIVE_MESSAGE,
                "code": "protocol_v2_not_active",
                "retryable": false
            }),
        );
    }

    assert_eq!(
        daemon
            .exchange(r#"{"type":"Status","request_id":"legacy-status"}"#)
            .await,
        concat!(
            r#"{"type":"StatusSnapshot","request_id":"legacy-status","status":{"active_leases":0,"pending_requests":0,"usage":[]}}"#,
            "\n"
        )
    );
}
