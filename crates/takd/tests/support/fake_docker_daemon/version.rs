use std::io;

use tokio::net::UnixStream;

use super::response::write_response;
use super::state::FakeDockerDaemonState;

pub(super) async fn write_version_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    if state.version_fails() {
        return write_response(
            stream,
            "500 Internal Server Error",
            "application/json",
            br#"{"message":"version failed"}"#,
        )
        .await;
    }
    let body = format!(
        r#"{{"Version":"test","ApiVersion":"1.47","Arch":"{}","Os":"linux"}}"#,
        state.daemon_arch()
    );
    write_response(stream, "200 OK", "application/json", body.as_bytes()).await
}
