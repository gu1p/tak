use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

const CONTAINER_ID: &str = "container-123";

#[rustfmt::skip]
pub struct NonzeroWaitDockerDaemon { socket_path: PathBuf, accept_task: JoinHandle<()> }

#[rustfmt::skip]
impl NonzeroWaitDockerDaemon {
    pub fn spawn(root: &Path, wait_status: i64, stderr_line: &str) -> Self {
        let socket_path = root.join("docker.sock");
        if socket_path.exists() { std::fs::remove_file(&socket_path).expect("remove stale fake docker socket"); }
        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let stderr_line = Arc::new(stderr_line.as_bytes().to_vec());
        let accept_task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                let stderr_line = Arc::clone(&stderr_line);
                tokio::spawn(async move { let _ = handle(stream, wait_status, &stderr_line).await; });
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
async fn handle(mut stream: UnixStream, wait_status: i64, stderr_line: &[u8]) -> io::Result<()> {
    let (method, path) = read_request(&mut stream).await?;
    let path = path.split_once('?').map_or(path.as_str(), |(path, _)| path);
    match (method.as_str(), path) {
        ("GET", p) if p.ends_with("/_ping") => write_response(&mut stream, "200 OK", b"OK").await,
        ("GET", p) if p.ends_with("/images/alpine:3.20/json") => write_response(&mut stream, "200 OK", br#"{"Id":"sha256:test-image"}"#).await,
        ("POST", p) if p.ends_with("/containers/create") => write_response(&mut stream, "201 Created", format!(r#"{{"Id":"{CONTAINER_ID}","Warnings":[]}}"#).as_bytes()).await,
        ("POST", p) if p.ends_with("/start") => write_empty_response(&mut stream, "204 No Content").await,
        ("DELETE", p) if p.contains("/containers/") => write_empty_response(&mut stream, "204 No Content").await,
        ("GET", p) if p.ends_with("/logs") => write_logs_response(&mut stream, stderr_line).await,
        ("POST", p) if p.ends_with("/wait") => write_response(&mut stream, "200 OK", format!(r#"{{"Error":null,"StatusCode":{wait_status}}}"#).as_bytes()).await,
        _ => write_response(&mut stream, "404 Not Found", b"not found").await,
    }
}

#[rustfmt::skip]
async fn read_request(stream: &mut UnixStream) -> io::Result<(String, String)> {
    let mut buffer = Vec::new();
    let header_end = loop {
        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") { break index; }
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "request ended before headers")); }
        buffer.extend_from_slice(&chunk[..bytes_read]);
    };
    let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let body_start = header_end + 4;
    let content_length = headers.lines().find_map(|line| line.split_once(':').and_then(|(name, value)| name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().expect("parse content-length")))).unwrap_or(0);
    while buffer.len() < body_start + content_length {
        let mut chunk = [0_u8; 1024];
        let read = stream.read(&mut chunk).await?;
        buffer.extend_from_slice(&chunk[..read]);
    }
    let mut parts = headers.lines().next().unwrap_or_default().split_whitespace();
    Ok((parts.next().unwrap_or_default().to_string(), parts.next().unwrap_or_default().to_string()))
}

#[rustfmt::skip]
async fn write_response(stream: &mut UnixStream, status: &str, body: &[u8]) -> io::Result<()> {
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

#[rustfmt::skip]
async fn write_empty_response(stream: &mut UnixStream, status: &str) -> io::Result<()> {
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    stream.flush().await
}

#[rustfmt::skip]
async fn write_logs_response(stream: &mut UnixStream, stderr_line: &[u8]) -> io::Result<()> {
    let mut frame = vec![2, 0, 0, 0];
    frame.extend_from_slice(&(stderr_line.len() as u32).to_be_bytes());
    frame.extend_from_slice(stderr_line);
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.docker.raw-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await?;
    stream.write_all(format!("{:X}\r\n", frame.len()).as_bytes()).await?;
    stream.write_all(&frame).await?;
    stream.write_all(b"\r\n0\r\n\r\n").await?;
    stream.flush().await
}
