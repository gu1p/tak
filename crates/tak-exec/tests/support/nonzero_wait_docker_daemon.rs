use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

#[path = "nonzero_wait_docker_daemon/wire.rs"]
mod wire;

const CONTAINER_ID: &str = "container-123";

use wire::{read_request, write_empty_response, write_logs_response, write_response};

#[rustfmt::skip]
pub struct NonzeroWaitDockerDaemon { socket_path: PathBuf, accept_task: JoinHandle<()> }

#[rustfmt::skip]
impl NonzeroWaitDockerDaemon {
    pub fn spawn(root: &Path, wait_status: i64, stderr_line: &str) -> Self {
        Self::spawn_with_wait_error(root, wait_status, None, stderr_line)
    }

    pub fn spawn_with_wait_error(
        root: &Path,
        wait_status: i64,
        wait_error: Option<&str>,
        stderr_line: &str,
    ) -> Self {
        let socket_path = root.join("docker.sock");
        if socket_path.exists() { std::fs::remove_file(&socket_path).expect("remove stale fake docker socket"); }
        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let wait_error = wait_error.map(ToOwned::to_owned);
        let stderr_line = Arc::new(stderr_line.as_bytes().to_vec());
        let accept_task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                let wait_error = wait_error.clone();
                let stderr_line = Arc::clone(&stderr_line);
                tokio::spawn(async move {
                    let _ = handle(stream, wait_status, wait_error.as_deref(), &stderr_line).await;
                });
            }
        });
        Self { socket_path, accept_task }
    }
    pub fn socket_path(&self) -> &Path { &self.socket_path }
}

#[rustfmt::skip]
impl Drop for NonzeroWaitDockerDaemon {
    fn drop(&mut self) { self.accept_task.abort(); let _ = std::fs::remove_file(&self.socket_path); }
}

#[rustfmt::skip]
async fn handle(
    mut stream: UnixStream,
    wait_status: i64,
    wait_error: Option<&str>,
    stderr_line: &[u8],
) -> io::Result<()> {
    let (method, path) = read_request(&mut stream).await?;
    let path = path.split_once('?').map_or(path.as_str(), |(path, _)| path);
    match (method.as_str(), path) {
        ("GET", p) if p.ends_with("/_ping") => write_response(&mut stream, "200 OK", b"OK").await,
        ("GET", p) if p.ends_with("/images/alpine:3.20/json") => write_response(&mut stream, "200 OK", br#"{"Id":"sha256:test-image"}"#).await,
        ("POST", p) if p.ends_with("/containers/create") => write_response(&mut stream, "201 Created", format!(r#"{{"Id":"{CONTAINER_ID}","Warnings":[]}}"#).as_bytes()).await,
        ("POST", p) if p.ends_with("/start") => write_empty_response(&mut stream, "204 No Content").await,
        ("DELETE", p) if p.contains("/containers/") => write_empty_response(&mut stream, "204 No Content").await,
        ("GET", p) if p.ends_with("/logs") => write_logs_response(&mut stream, stderr_line).await,
        ("POST", p) if p.ends_with("/wait") => write_response(&mut stream, "200 OK", wait_response_body(wait_status, wait_error).as_bytes()).await,
        _ => write_response(&mut stream, "404 Not Found", b"not found").await,
    }
}

#[rustfmt::skip]
fn wait_response_body(wait_status: i64, wait_error: Option<&str>) -> String {
    match wait_error {
        Some(message) => format!(
            r#"{{"StatusCode":{wait_status},"Error":{{"Message":"{message}"}}}}"#
        ),
        None => format!(r#"{{"Error":null,"StatusCode":{wait_status}}}"#),
    }
}
