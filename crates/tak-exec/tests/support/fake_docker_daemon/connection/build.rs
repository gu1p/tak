use std::io;

use tokio::net::UnixStream;

use super::super::build::parse_build_request;
use super::super::request::FakeDockerRequest;
use super::super::response::write_response;
use super::super::state::FakeDockerDaemonState;

pub(super) async fn write_build_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    state.record_build(parse_build_request(request)?);
    if let Some(message) = state.build_failure_message() {
        let body = format!(
            "{}\n{}\n",
            serde_json::json!({ "stream": "Step 1/1 : RUN failing build step\n" }),
            serde_json::json!({
                "error": message,
                "errorDetail": { "message": message },
            })
        );
        return write_response(stream, "200 OK", "application/json", body.as_bytes()).await;
    }
    write_response(
        stream,
        "200 OK",
        "application/json",
        br#"{"stream":"Successfully built sha256:test-image\n"}"#,
    )
    .await
}
