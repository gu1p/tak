use std::io;

use tokio::net::UnixStream;

use super::create::create_container;
use super::request::FakeDockerRequest;
use super::response::write_response;
use super::state::FakeDockerDaemonState;

pub(super) async fn write_image_status(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    image: &str,
) -> io::Result<()> {
    let (status, body) = if state.image_present(image) {
        ("200 OK", br#"{"Id":"sha256:test-image"}"#.as_slice())
    } else {
        ("404 Not Found", br#"{"message":"not found"}"#.as_slice())
    };
    write_response(stream, status, "application/json", body).await
}

pub(super) async fn write_version_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    let body = format!(
        r#"{{"Version":"test","ApiVersion":"1.47","Arch":"{}","Os":"linux"}}"#,
        state.daemon_arch()
    );
    write_response(stream, "200 OK", "application/json", body.as_bytes()).await
}

pub(super) async fn write_pull_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    let image = request.query_param("fromImage").unwrap_or_default();
    state.mark_image_pulled(&image);
    write_response(
        stream,
        "200 OK",
        "application/json",
        format!(r#"{{"status":"pulled {image}"}}"#).as_bytes(),
    )
    .await
}

pub(super) async fn write_build_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    if let Some(image) = request.query_param("t") {
        state.mark_image_built(&image);
    }
    write_response(
        stream,
        "200 OK",
        "application/json",
        br#"{"stream":"Successfully built sha256:test-image\n"}"#,
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
