use std::io;

use tokio::net::UnixStream;

use super::super::response::{write_empty_response, write_logs_response, write_response};
use super::super::state::FakeDockerDaemonState;

pub(super) async fn write_container_start_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    if let Some(message) = state.start_failure_message() {
        write_response(
            stream,
            "500 Internal Server Error",
            "application/json",
            format!(r#"{{"message":"{message}"}}"#).as_bytes(),
        )
        .await
    } else {
        write_empty_response(stream, "204 No Content").await
    }
}

pub(super) async fn write_container_logs_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    if let Some(message) = state.logs_failure_message() {
        write_response(
            stream,
            "500 Internal Server Error",
            "application/json",
            format!(r#"{{"message":"{message}"}}"#).as_bytes(),
        )
        .await
    } else {
        write_logs_response(stream, state).await
    }
}

pub(super) async fn write_container_wait_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    state.wait_until_released().await;
    write_response(stream, "200 OK", "application/json", br#"{"StatusCode":0}"#).await
}

pub(super) async fn write_container_remove_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    path: &str,
) -> io::Result<()> {
    if let Some(container_id) = path
        .split_once("/containers/")
        .and_then(|(_, tail)| tail.split('/').next())
    {
        state.record_remove(container_id.to_string());
    }
    if let Some(message) = state.container_removal_failure_message() {
        return write_response(
            stream,
            "500 Internal Server Error",
            "application/json",
            format!(r#"{{"message":"{message}"}}"#).as_bytes(),
        )
        .await;
    }
    write_empty_response(stream, "204 No Content").await
}
