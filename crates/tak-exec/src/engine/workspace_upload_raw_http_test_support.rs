#![cfg(test)]

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub(super) struct RawRequest {
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

pub(super) async fn read_raw_request(stream: &mut TcpStream) -> Option<RawRequest> {
    let mut bytes = Vec::new();
    let header_end = read_headers(stream, &mut bytes).await?;
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let length = content_length(&headers);
    let mut body = bytes[header_end..].to_vec();
    while body.len() < length {
        let mut chunk = vec![0_u8; length - body.len()];
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    Some(RawRequest {
        path: request_path(&headers)?,
        body,
    })
}

async fn read_headers(stream: &mut TcpStream, bytes: &mut Vec<u8>) -> Option<usize> {
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        let previous_len = bytes.len();
        bytes.extend_from_slice(&chunk[..read]);
        let search_start = previous_len.saturating_sub(3);
        if let Some(index) = bytes[search_start..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            return Some(search_start + index + 4);
        }
    }
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0)
}

fn request_path(headers: &str) -> Option<String> {
    headers
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
}
