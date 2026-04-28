use std::io;
use std::sync::Arc;

use tokio::net::UnixStream;

use super::build::parse_build_request;
use super::create::parse_create_request;
use super::image_delete::write_image_delete_response;
use super::request::read_request;
use super::response::{write_empty_response, write_logs_response, write_response};
use super::state::FakeDockerDaemonState;
use super::{CONTAINER_ID, PullRecord};

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
            state.record_build(parse_build_request(&request)?);
            write_response(
                &mut stream,
                "200 OK",
                "application/json",
                br#"{"stream":"Successfully built sha256:test-image\n"}"#,
            )
            .await?;
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
            write_empty_response(&mut stream, "204 No Content").await?
        }
        "GET" if path.ends_with("/logs") => write_logs_response(&mut stream, &state).await?,
        "POST" if path.ends_with("/wait") => {
            state.wait_until_released().await;
            write_response(
                &mut stream,
                "200 OK",
                "application/json",
                br#"{"StatusCode":0}"#,
            )
            .await?;
        }
        "DELETE" if path.contains("/containers/") => {
            write_empty_response(&mut stream, "204 No Content").await?
        }
        "DELETE" if path.contains("/images/") => {
            write_image_delete_response(&mut stream, &state, &request).await?
        }
        _ => write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?,
    }

    Ok(())
}
