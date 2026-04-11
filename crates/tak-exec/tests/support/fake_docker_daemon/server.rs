use std::io;
use std::sync::Arc;

use tokio::net::{UnixListener, UnixStream};

use super::request::{FakeDockerRequest, read_request};
use super::response::{write_empty_response, write_logs_response, write_response};
use super::tar::tar_file_entries;
use super::{BuildRecord, CONTAINER_ID, FakeDockerDaemonState, IMAGE_ID};

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
        "GET" if path.ends_with("/images/alpine:3.20/json") => {
            let body = format!(r#"{{"Id":"{IMAGE_ID}"}}"#);
            write_response(&mut stream, "200 OK", "application/json", body.as_bytes()).await?;
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
        "POST" if path.ends_with("/containers/create") => {
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
        _ => write_response(&mut stream, "404 Not Found", "text/plain", b"not found").await?,
    }

    Ok(())
}

fn parse_build_request(request: &FakeDockerRequest) -> io::Result<BuildRecord> {
    Ok(BuildRecord {
        dockerfile: request
            .query_param("dockerfile")
            .unwrap_or_else(|| "Dockerfile".to_string()),
        context_entries: tar_file_entries(&request.body)?,
    })
}
