use std::io;

use tokio::net::UnixStream;

use super::create::create_container;
use super::request::FakeDockerRequest;
use super::response::write_response;
use super::state::FakeDockerDaemonState;

pub(super) async fn write_image_status(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    let (status, body) = if state.image_present() {
        ("200 OK", br#"{"Id":"sha256:test-image"}"#.as_slice())
    } else {
        ("404 Not Found", br#"{"message":"not found"}"#.as_slice())
    };
    write_response(stream, status, "application/json", body).await
}

pub(super) async fn write_pull_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    state.mark_image_pulled();
    write_response(
        stream,
        "200 OK",
        "application/json",
        br#"{"status":"pulled alpine:3.20"}"#,
    )
    .await
}

pub(super) async fn write_create_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    let created = create_container(state, request)?;
    let body = format!(
        r#"{{"Id":"{}","Warnings":[]}}"#,
        created.record.container_id
    );
    state.record_create(created.record, created.exit_code);
    write_response(stream, "201 Created", "application/json", body.as_bytes()).await
}

pub(super) async fn write_wait_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    path: &str,
) -> io::Result<()> {
    let container_id = path
        .split("/containers/")
        .nth(1)
        .and_then(|tail| tail.split('/').next())
        .unwrap_or_default();
    let body = format!(
        r#"{{"StatusCode":{}}}"#,
        state.container_exit_code(container_id)
    );
    write_response(stream, "200 OK", "application/json", body.as_bytes()).await
}
