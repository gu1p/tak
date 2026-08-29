use crate::support::raw_local_protocol::RawLocalProtocol;

const INVALID_MESSAGE: &str = "protocol_version must appear exactly once as the integer 2.";
const UNSUPPORTED_MESSAGE: &str =
    "This takd requires protocol v2. Upgrade tak, takd, and workers together.";

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_invalid_and_unsupported_versions_are_rejected_without_fallback() {
    let mut daemon = RawLocalProtocol::start().await;
    let invalid = [
        r#"{"protocol_version":2,"protocol_version":2,"request_id":"duplicate","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":2,"protocol\u005fversion":2,"request_id":"escaped","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":null,"request_id":"null","operation":{"type":"ListRuns"}}"#,
    ];

    for (request, request_id) in invalid.into_iter().zip(["duplicate", "escaped", "null"]) {
        let response = daemon.exchange(request).await;
        super::assert_json_response(
            &response,
            version_error(request_id, INVALID_MESSAGE, "protocol_version_invalid"),
        );
    }

    for (version, request_id) in [(1, "old"), (3, "new")] {
        let request = format!(
            r#"{{"protocol_version":{version},"request_id":"{request_id}","operation":{{"type":"ListRuns"}}}}"#
        );
        let response = daemon.exchange(&request).await;
        super::assert_json_response(
            &response,
            version_error(
                request_id,
                UNSUPPORTED_MESSAGE,
                "protocol_version_unsupported",
            ),
        );
    }
}

fn version_error(request_id: &str, message: &str, code: &str) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": 2,
        "type": "Error",
        "request_id": request_id,
        "message": message,
        "code": code,
        "retryable": false
    })
}
