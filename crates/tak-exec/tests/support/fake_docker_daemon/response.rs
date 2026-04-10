use std::io;

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use super::{FakeDockerDaemonState, LOG_MESSAGE};

pub(super) async fn write_logs_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\n\
Content-Type: application/vnd.docker.raw-stream\r\n\
Transfer-Encoding: chunked\r\n\
Connection: close\r\n\
\r\n",
        )
        .await?;
    write_chunk(stream, &docker_stdout_frame(LOG_MESSAGE)).await?;
    stream.flush().await?;
    state.wait_until_released().await;
    stream.write_all(b"0\r\n\r\n").await?;
    stream.flush().await
}

pub(super) async fn write_response(
    stream: &mut UnixStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

pub(super) async fn write_empty_response(stream: &mut UnixStream, status: &str) -> io::Result<()> {
    let headers = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(headers.as_bytes()).await?;
    stream.flush().await
}

async fn write_chunk(stream: &mut UnixStream, chunk: &[u8]) -> io::Result<()> {
    let header = format!("{:X}\r\n", chunk.len());
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(chunk).await?;
    stream.write_all(b"\r\n").await
}

fn docker_stdout_frame(message: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(message.len() + 8);
    frame.push(1);
    frame.extend_from_slice(&[0, 0, 0]);
    frame.extend_from_slice(&(message.len() as u32).to_be_bytes());
    frame.extend_from_slice(message);
    frame
}
