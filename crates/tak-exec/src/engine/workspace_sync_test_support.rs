#![cfg(test)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use super::SyncedOutput;
use super::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};

pub(super) struct RangeServer {
    pub(super) addr: String,
    ranges: Arc<Mutex<Vec<String>>>,
    dropped: Arc<AtomicBool>,
}

impl RangeServer {
    pub(super) async fn spawn(body: Vec<u8>) -> Self {
        Self::spawn_with_drop(body, true).await
    }

    pub(super) async fn spawn_without_drop(body: Vec<u8>) -> Self {
        Self::spawn_with_drop(body, false).await
    }

    async fn spawn_with_drop(body: Vec<u8>, drop_first_response: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(!drop_first_response));
        spawn_range_server(listener, body, Arc::clone(&ranges), Arc::clone(&dropped));
        Self {
            addr,
            ranges,
            dropped,
        }
    }

    pub(super) async fn ranges(&self) -> Vec<String> {
        self.ranges.lock().await.clone()
    }

    pub(super) fn dropped_response(&self) -> bool {
        self.dropped.load(Ordering::SeqCst)
    }
}

pub(super) fn direct_target(addr: &str) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: format!("http://{addr}"),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}

pub(super) fn synced_output(path: &str, body: &[u8]) -> SyncedOutput {
    SyncedOutput {
        path: path.into(),
        digest: format!("sha256:{:x}", Sha256::digest(body)),
        size_bytes: body.len() as u64,
    }
}

fn spawn_range_server(
    listener: TcpListener,
    body: Vec<u8>,
    ranges: Arc<Mutex<Vec<String>>>,
    dropped: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        let body = Arc::new(body);
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(serve_range_connection(
                stream,
                Arc::clone(&body),
                Arc::clone(&ranges),
                Arc::clone(&dropped),
            ));
        }
    });
}

async fn serve_range_connection(
    mut stream: TcpStream,
    body: Arc<Vec<u8>>,
    ranges: Arc<Mutex<Vec<String>>>,
    dropped: Arc<AtomicBool>,
) {
    let Some(headers) = read_headers(&mut stream).await else {
        return;
    };
    let range = header_value(&headers, "range").unwrap_or_default();
    ranges.lock().await.push(range.clone());
    if !dropped.swap(true, Ordering::SeqCst) {
        return;
    }
    let offset = range_offset(&range);
    let response = range_response(&body, offset);
    let _ = stream.write_all(&response).await;
    let _ = stream.shutdown().await;
}

async fn read_headers(stream: &mut TcpStream) -> Option<String> {
    let mut bytes = Vec::new();
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
            let header_end = search_start + index + 4;
            return Some(String::from_utf8_lossy(&bytes[..header_end]).to_string());
        }
    }
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    headers.lines().find_map(|line| {
        let (header_name, value) = line.split_once(':')?;
        header_name
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
}

fn range_offset(range: &str) -> usize {
    range
        .strip_prefix("bytes=")
        .and_then(|value| value.split_once('-').map(|(start, _)| start))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

fn range_response(body: &[u8], offset: usize) -> Vec<u8> {
    let end = range_end(body, offset);
    let slice = &body[offset..=end];
    let mut response = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
        slice.len(), offset, end, body.len()
    )
    .into_bytes();
    response.extend_from_slice(slice);
    response
}

fn range_end(body: &[u8], offset: usize) -> usize {
    body.len()
        .saturating_sub(1)
        .min(offset + 8 * 1024 * 1024 - 1)
}
