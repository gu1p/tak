use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

#[rustfmt::skip]
pub(super) async fn read_request(stream: &mut UnixStream) -> io::Result<(String, String)> {
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
pub(super) async fn write_response(
    stream: &mut UnixStream,
    status: &str,
    body: &[u8],
) -> io::Result<()> {
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

#[rustfmt::skip]
pub(super) async fn write_empty_response(
    stream: &mut UnixStream,
    status: &str,
) -> io::Result<()> {
    stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    stream.flush().await
}

#[rustfmt::skip]
pub(super) async fn write_logs_response(
    stream: &mut UnixStream,
    stderr_line: &[u8],
) -> io::Result<()> {
    let mut frame = vec![2, 0, 0, 0];
    frame.extend_from_slice(&(stderr_line.len() as u32).to_be_bytes());
    frame.extend_from_slice(stderr_line);
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.docker.raw-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await?;
    stream.write_all(format!("{:X}\r\n", frame.len()).as_bytes()).await?;
    stream.write_all(&frame).await?;
    stream.write_all(b"\r\n0\r\n\r\n").await?;
    stream.flush().await
}
