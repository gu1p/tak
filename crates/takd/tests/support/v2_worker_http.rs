use serde::de::DeserializeOwned;

use super::worker_http::{RawHttpResponse, RunningServer, send_raw_request};

pub async fn post(
    server: &RunningServer,
    path: &str,
    bearer: Option<&str>,
    versions: &[&str],
    body: &[u8],
) -> RawHttpResponse {
    let authorization = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let version_headers = versions
        .iter()
        .map(|value| format!("X-Tak-Protocol-Version: {value}\r\n"))
        .collect::<String>();
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {}\r\n{authorization}{version_headers}\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        server.addr,
        body.len(),
    );
    let mut request = head.into_bytes();
    request.extend_from_slice(body);
    send_raw_request(server.addr, &request).await
}

pub async fn get(
    server: &RunningServer,
    path: &str,
    bearer: Option<&str>,
    versions: &[&str],
) -> RawHttpResponse {
    let authorization = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let version_headers = versions
        .iter()
        .map(|value| format!("X-Tak-Protocol-Version: {value}\r\n"))
        .collect::<String>();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\n{authorization}{version_headers}\
         Connection: close\r\n\r\n",
        server.addr,
    );
    send_raw_request(server.addr, request.as_bytes()).await
}

pub fn status(response: &RawHttpResponse) -> u16 {
    response
        .head
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}

pub fn json<T: DeserializeOwned>(response: &RawHttpResponse) -> T {
    serde_json::from_slice(&response.body).unwrap()
}
