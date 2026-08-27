use std::io;
use std::sync::Arc;

use tokio::net::{UnixListener, UnixStream};

use super::handlers::{
    write_build_response, write_create_response, write_image_status, write_pull_response,
    write_wait_response,
};
use super::request::{FakeDockerRequest, read_request};
use super::response::{write_empty_response, write_logs_response, write_response};
use super::state::FakeDockerDaemonState;
use super::stats::write_stats_response;
use super::version::write_version_response;

mod accept;
mod container_routes;
mod image_routes;

pub(super) use accept::run_fake_docker_daemon;
use container_routes::{
    write_container_inspect_response, write_container_list_response, write_unpause_failure_response,
};
use image_routes::requested_image_name;

async fn handle_connection(
    mut stream: UnixStream,
    state: Arc<FakeDockerDaemonState>,
) -> io::Result<()> {
    let request = read_request(&mut stream).await?;
    let path = request.path_without_query();

    match request.method.as_str() {
        "GET" if path.ends_with("/_ping") => {
            tokio::time::sleep(state.ping_response_delay).await;
            write_response(&mut stream, "200 OK", "text/plain", b"OK").await?
        }
        "GET" if path.ends_with("/version") => write_version_response(&mut stream, &state).await?,
        "GET" if path.ends_with("/containers/json") => {
            write_container_list_response(&mut stream, &state, &request).await?
        }
        "GET" if path.contains("/containers/") && path.ends_with("/json") => {
            write_container_inspect_response(&mut stream, &state, path).await?
        }
        "GET" if path.contains("/images/") && path.ends_with("/json") => {
            let Some(image) = requested_image_name(&request) else {
                write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?;
                return Ok(());
            };
            write_image_status(&mut stream, &state, &image).await?
        }
        "POST" if path.ends_with("/images/create") => {
            write_pull_response(&mut stream, &state, &request).await?
        }
        "POST" if path.ends_with("/build") => {
            write_build_response(&mut stream, &state, &request).await?
        }
        "POST" if path.ends_with("/containers/create") => {
            write_create_response(&mut stream, &state, &request).await?
        }
        "POST" if path.ends_with("/start") => {
            write_empty_response(&mut stream, "204 No Content").await?
        }
        "POST" if path.ends_with("/unpause") => {
            write_unpause_failure_response(&mut stream, &state, path).await?
        }
        "GET" if path.ends_with("/logs") => write_logs_response(&mut stream).await?,
        "GET" if path.ends_with("/stats") => write_stats_response(&mut stream, &state).await?,
        "POST" if path.ends_with("/wait") => write_wait_response(&mut stream, &state, path).await?,
        "DELETE" if path.contains("/containers/") => {
            if let Some(container_id) = path
                .split_once("/containers/")
                .and_then(|(_, tail)| tail.split('/').next())
            {
                if !state.record_container_removal_attempt(container_id) {
                    write_response(
                        &mut stream,
                        "500 Internal Server Error",
                        "application/json",
                        br#"{"message":"injected removal failure"}"#,
                    )
                    .await?;
                    return Ok(());
                }
                state.record_container_removed(container_id);
            }
            write_empty_response(&mut stream, "204 No Content").await?
        }
        _ => write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?,
    }

    Ok(())
}
