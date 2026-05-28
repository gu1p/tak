#![cfg(test)]
use super::workspace_upload_raw_http_test_support::{RawRequest, read_raw_request};
use prost::Message;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tak_proto::{
    AppendWorkspaceUploadResponse, BeginWorkspaceUploadRequest, BeginWorkspaceUploadResponse,
    FinishWorkspaceUploadResponse,
};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

pub(super) struct UploadServer {
    pub(super) addr: String,
    state: Arc<Mutex<UploadState>>,
    dropped: Arc<AtomicBool>,
}
struct UploadState {
    bytes: Vec<u8>,
    expected_size: u64,
    short_first_append: Option<usize>,
    finish_conflict_sent: bool,
}

impl UploadServer {
    pub(super) async fn spawn() -> Self {
        Self::spawn_with_short_first_append(None).await
    }

    pub(super) async fn spawn_finish_conflict(short_first_append: usize) -> Self {
        Self::spawn_with_short_first_append(Some(short_first_append)).await
    }

    async fn spawn_with_short_first_append(short_first_append: Option<usize>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let state = Arc::new(Mutex::new(UploadState {
            bytes: Vec::new(),
            expected_size: 0,
            short_first_append,
            finish_conflict_sent: false,
        }));
        let dropped = Arc::new(AtomicBool::new(short_first_append.is_some()));
        spawn_upload_server(listener, Arc::clone(&state), Arc::clone(&dropped));
        Self {
            addr,
            state,
            dropped,
        }
    }
    pub(super) async fn bytes(&self) -> Vec<u8> {
        self.state.lock().await.bytes.clone()
    }

    pub(super) fn dropped_response(&self) -> bool {
        self.dropped.load(Ordering::SeqCst)
    }
}
fn spawn_upload_server(
    listener: TcpListener,
    state: Arc<Mutex<UploadState>>,
    dropped: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(serve_upload_connection(
                stream,
                Arc::clone(&state),
                Arc::clone(&dropped),
            ));
        }
    });
}

async fn serve_upload_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<UploadState>>,
    dropped: Arc<AtomicBool>,
) {
    let Some(request) = read_raw_request(&mut stream).await else {
        return;
    };
    let Some(response) = handle_upload_request(request, state, dropped).await else {
        return;
    };
    let _ = stream.write_all(&response).await;
    let _ = stream.shutdown().await;
}

async fn handle_upload_request(
    request: RawRequest,
    state: Arc<Mutex<UploadState>>,
    dropped: Arc<AtomicBool>,
) -> Option<Vec<u8>> {
    if request.path == "/v2/workspaces/uploads/begin" {
        return Some(begin_response(request, state).await);
    }
    if request.path == "/v2/workspaces/uploads/upload-1/finish" {
        return Some(finish_response(state).await);
    }
    append_response(request, state, dropped).await
}

async fn begin_response(request: RawRequest, state: Arc<Mutex<UploadState>>) -> Vec<u8> {
    let request = BeginWorkspaceUploadRequest::decode(request.body.as_slice()).expect("begin");
    let mut state = state.lock().await;
    state.expected_size = request.size_bytes;
    let offset = state.bytes.len() as u64;
    protobuf_response(
        200,
        BeginWorkspaceUploadResponse {
            upload_id: "upload-1".into(),
            offset,
            complete: false,
        },
    )
}

async fn finish_response(state: Arc<Mutex<UploadState>>) -> Vec<u8> {
    let mut state = state.lock().await;
    let size_bytes = state.bytes.len() as u64;
    if size_bytes < state.expected_size && !state.finish_conflict_sent {
        state.finish_conflict_sent = true;
        return append_offset_response(409, size_bytes);
    }
    protobuf_response(
        200,
        FinishWorkspaceUploadResponse {
            upload_id: "upload-1".into(),
            size_bytes,
            complete: true,
        },
    )
}

async fn append_response(
    request: RawRequest,
    state: Arc<Mutex<UploadState>>,
    dropped: Arc<AtomicBool>,
) -> Option<Vec<u8>> {
    let offset = request
        .path
        .strip_prefix("/v2/workspaces/uploads/upload-1?offset=")?
        .parse::<u64>()
        .ok()?;
    let mut state = state.lock().await;
    if !dropped.swap(true, Ordering::SeqCst) {
        state.bytes.extend_from_slice(&request.body);
        return None;
    }
    if offset != state.bytes.len() as u64 {
        return Some(append_offset_response(409, state.bytes.len() as u64));
    }
    let reported_offset = offset + request.body.len() as u64;
    if let Some(limit) = state.short_first_append.take() {
        state
            .bytes
            .extend_from_slice(&request.body[..request.body.len().min(limit)]);
        return Some(append_offset_response(200, reported_offset));
    }
    state.bytes.extend_from_slice(&request.body);
    Some(append_offset_response(200, reported_offset))
}

fn append_offset_response(status: u16, offset: u64) -> Vec<u8> {
    protobuf_response(
        status,
        AppendWorkspaceUploadResponse {
            offset,
            complete: false,
        },
    )
}

fn protobuf_response<T: Message>(status: u16, message: T) -> Vec<u8> {
    let body = message.encode_to_vec();
    let mut response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        reason(status),
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        409 => "Conflict",
        _ => "Unknown",
    }
}
