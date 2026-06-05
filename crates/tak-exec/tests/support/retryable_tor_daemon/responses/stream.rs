use prost::Message;
use tak_proto::AppendWorkspaceUploadResponse;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

use super::super::State;

pub(in crate::support::retryable_tor_daemon) fn content_length(headers: &str) -> usize {
    header_value(headers, "content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

pub(in crate::support::retryable_tor_daemon) async fn read_headers(
    reader: &mut BufReader<UnixStream>,
) -> String {
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        let done = reader.read_line(&mut line).await.unwrap_or(0) == 0;
        if done || line.trim_end().is_empty() {
            break;
        }
        headers.push_str(&line);
    }
    headers
}

pub(in crate::support::retryable_tor_daemon) fn record_stream(
    first_line: &str,
    headers: &str,
    state: &mut State,
) -> bool {
    let upload_id = upload_id(first_line);
    let offset = stream_offset(first_line);
    state.upload_ids.push(upload_id);
    state.stream_offsets.push(offset);
    state.size = header_value(headers, "x-tak-upload-size")
        .and_then(|value| value.parse().ok())
        .unwrap_or(state.size);
    if state.committed == 0 && offset == 0 {
        state.committed = state.size.min(8);
        return false;
    }
    if offset < state.committed {
        return true;
    }
    if state.drops_at_committed_offset < 4 {
        state.drops_at_committed_offset += 1;
        return false;
    }
    state.committed = state.size;
    true
}

pub(in crate::support::retryable_tor_daemon) fn stream_response(state: &State) -> Vec<u8> {
    let body = AppendWorkspaceUploadResponse {
        offset: state.committed,
        complete: state.committed == state.size,
    }
    .encode_to_vec();
    let mut response =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
    response.extend(body);
    response
}

fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then_some(value.trim())
    })
}

fn stream_offset(first_line: &str) -> u64 {
    first_line
        .split("offset=")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn upload_id(first_line: &str) -> String {
    first_line
        .split("/v2/workspaces/uploads/")
        .nth(1)
        .and_then(|value| value.split('/').next())
        .unwrap_or("")
        .to_string()
}
