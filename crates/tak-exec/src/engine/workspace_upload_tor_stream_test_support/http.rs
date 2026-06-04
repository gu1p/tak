use prost::Message;
use tokio::io::AsyncBufReadExt;
use tokio::net::UnixStream;

pub(super) async fn read_headers(reader: &mut tokio::io::BufReader<UnixStream>) -> String {
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
            break;
        }
        if line.trim_end().is_empty() {
            break;
        }
        headers.push_str(&line);
    }
    headers
}

pub(super) fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0)
}

pub(super) fn stream_offset(first_line: &str) -> Option<u64> {
    first_line
        .split_whitespace()
        .nth(1)?
        .split_once("offset=")?
        .1
        .parse()
        .ok()
}

pub(super) fn protobuf_http_response<T: Message>(message: T) -> Vec<u8> {
    let body = message.encode_to_vec();
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

pub(super) fn json_string_field(request: &str, field: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(request)
        .ok()?
        .get(field)?
        .as_str()
        .map(str::to_string)
}

pub(super) fn peers_response() -> serde_json::Value {
    serde_json::json!({
        "type": "PeersSnapshot",
        "peers": [{
            "node_id": "builder-selected",
            "endpoint": "http://builder-selected.onion"
        }]
    })
}
