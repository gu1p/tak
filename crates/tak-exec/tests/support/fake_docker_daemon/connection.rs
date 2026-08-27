use std::io;
use std::sync::Arc;

use tokio::net::UnixStream;

use super::create::parse_create_request;
use super::image_delete::write_image_delete_response;
use super::request::read_request;
use super::response::write_response;
use super::state::FakeDockerDaemonState;
use super::{CONTAINER_ID, PullRecord};

mod build;
mod container;

use build::write_build_response;
use container::{
    write_container_logs_response, write_container_remove_response, write_container_start_response,
    write_container_wait_response,
};

pub(super) async fn handle_connection(
    mut stream: UnixStream,
    state: Arc<FakeDockerDaemonState>,
) -> io::Result<()> {
    let request = read_request(&mut stream).await?;
    let path = request.path_without_query();

    match request.method.as_str() {
        "GET" if path.ends_with("/_ping") => {
            write_response(&mut stream, "200 OK", "text/plain", b"OK").await?
        }
        "GET" if path.contains("/images/") && path.ends_with("/json") => {
            let Some(image) = request.requested_image_name() else {
                write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?;
                return Ok(());
            };
            if let Some(info) = state.image_info(&image) {
                let body = format!(r#"{{"Id":"{}","Size":{}}}"#, info.id, info.size);
                write_response(&mut stream, "200 OK", "application/json", body.as_bytes()).await?;
            } else {
                write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?;
            }
        }
        "POST" if path.ends_with("/build") => {
            write_build_response(&mut stream, &state, &request).await?
        }
        "POST" if path.ends_with("/images/create") => {
            let image = request.pull_image_name().unwrap_or_default();
            state.record_pull(PullRecord { image });
            write_response(
                &mut stream,
                "200 OK",
                "application/json",
                br#"{"status":"pulled"}"#,
            )
            .await?;
        }
        "POST" if path.ends_with("/containers/create") => {
            state.record_create(parse_create_request(&request)?);
            let body = format!(r#"{{"Id":"{CONTAINER_ID}","Warnings":[]}}"#);
            write_response(
                &mut stream,
                "201 Created",
                "application/json",
                body.as_bytes(),
            )
            .await?;
        }
        "POST" if path.ends_with("/start") => {
            write_container_start_response(&mut stream, &state).await?
        }
        "GET" if path.ends_with("/logs") => {
            write_container_logs_response(&mut stream, &state).await?
        }
        "POST" if path.ends_with("/wait") => {
            write_container_wait_response(&mut stream, &state).await?
        }
        "DELETE" if path.contains("/containers/") => {
            write_container_remove_response(&mut stream, &state, path).await?
        }
        "DELETE" if path.contains("/images/") => {
            write_image_delete_response(&mut stream, &state, &request).await?
        }
        _ => write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?,
    }

    Ok(())
}
