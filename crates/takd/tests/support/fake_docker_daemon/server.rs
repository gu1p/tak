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
use super::version::write_version_response;

pub(super) async fn run_fake_docker_daemon(
    listener: UnixListener,
    state: Arc<FakeDockerDaemonState>,
) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _ = handle_connection(stream, state).await;
        });
    }
}

async fn handle_connection(
    mut stream: UnixStream,
    state: Arc<FakeDockerDaemonState>,
) -> io::Result<()> {
    let request = read_request(&mut stream).await?;
    let path = request.path_without_query();

    match request.method.as_str() {
        "GET" if path.ends_with("/_ping") => {
            write_response(&mut stream, "200 OK", "text/plain", b"OK").await?
        }
        "GET" if path.ends_with("/version") => write_version_response(&mut stream, &state).await?,
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
        "GET" if path.ends_with("/logs") => write_logs_response(&mut stream).await?,
        "POST" if path.ends_with("/wait") => write_wait_response(&mut stream, &state, path).await?,
        "DELETE" if path.contains("/containers/") => {
            write_empty_response(&mut stream, "204 No Content").await?
        }
        _ => write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?,
    }

    Ok(())
}

fn requested_image_name(request: &FakeDockerRequest) -> Option<String> {
    let path = request.path_without_query();
    let tail = path.split("/images/").nth(1)?;
    let image = tail.strip_suffix("/json")?;
    Some(
        image
            .replace("%3A", ":")
            .replace("%2F", "/")
            .replace("%40", "@"),
    )
}
