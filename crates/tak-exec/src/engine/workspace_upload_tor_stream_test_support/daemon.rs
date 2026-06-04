use prost::Message;
use std::collections::VecDeque;
use std::sync::Arc;
use tak_proto::{AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

use super::env::TakdSocketEnv;
use super::http::{
    content_length, json_string_field, peers_response, protobuf_http_response, read_headers,
    stream_offset,
};

pub(crate) struct TorStreamUploadDaemon {
    state: Arc<Mutex<UploadState>>,
    _temp_dir: tempfile::TempDir,
    _socket_env: TakdSocketEnv,
    task: tokio::task::JoinHandle<()>,
}

struct UploadState {
    bytes: Vec<u8>,
    expected_size: u64,
    dropped_commits: VecDeque<usize>,
    always_drop_without_progress: bool,
    stream_offsets: Vec<u64>,
    status_nodes: Vec<String>,
}

impl TorStreamUploadDaemon {
    pub(crate) async fn spawn_with_dropped_commits(
        archive: &[u8],
        dropped_commits: Vec<usize>,
    ) -> Self {
        spawn_daemon(archive, dropped_commits, false).await
    }

    pub(crate) async fn spawn_without_progress(archive: &[u8]) -> Self {
        spawn_daemon(archive, Vec::new(), true).await
    }

    pub(crate) async fn bytes(&self) -> Vec<u8> {
        self.state.lock().await.bytes.clone()
    }

    pub(crate) async fn stream_offsets(&self) -> Vec<u64> {
        self.state.lock().await.stream_offsets.clone()
    }

    pub(crate) async fn status_nodes(&self) -> Vec<String> {
        self.state.lock().await.status_nodes.clone()
    }
}

impl Drop for TorStreamUploadDaemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn_daemon(
    archive: &[u8],
    dropped_commits: Vec<usize>,
    always_drop_without_progress: bool,
) -> TorStreamUploadDaemon {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let socket_env = TakdSocketEnv::set(&socket_path);
    let listener = UnixListener::bind(&socket_path).expect("bind fake daemon");
    let state = Arc::new(Mutex::new(UploadState {
        bytes: Vec::new(),
        expected_size: archive.len() as u64,
        dropped_commits: dropped_commits.into(),
        always_drop_without_progress,
        stream_offsets: Vec::new(),
        status_nodes: Vec::new(),
    }));
    let task = tokio::spawn(serve_daemon(listener, Arc::clone(&state)));
    TorStreamUploadDaemon {
        state,
        _temp_dir: temp,
        _socket_env: socket_env,
        task,
    }
}

async fn serve_daemon(listener: UnixListener, state: Arc<Mutex<UploadState>>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(serve_connection(stream, Arc::clone(&state)));
    }
}

async fn serve_connection(stream: UnixStream, state: Arc<Mutex<UploadState>>) {
    let mut reader = BufReader::new(stream);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).await.unwrap_or(0) == 0 {
        return;
    }
    if first_line.starts_with('{') {
        serve_daemon_request(reader, first_line, state).await;
        return;
    }
    serve_http_request(reader, first_line, state).await;
}

async fn serve_daemon_request(
    mut reader: BufReader<UnixStream>,
    first_line: String,
    state: Arc<Mutex<UploadState>>,
) {
    let response = if first_line.contains(r#""type":"PeersEligible""#) {
        peers_response()
    } else if first_line.contains(r#""type":"ForwardRemoteHttp""#) {
        status_response(&first_line, state).await
    } else {
        serde_json::json!({"type": "Error", "message": "unexpected request"})
    };
    let stream = reader.get_mut();
    let _ = stream.write_all(response.to_string().as_bytes()).await;
    let _ = stream.write_all(b"\n").await;
}

async fn serve_http_request(
    mut reader: BufReader<UnixStream>,
    first_line: String,
    state: Arc<Mutex<UploadState>>,
) {
    let headers = read_headers(&mut reader).await;
    let mut body = vec![0_u8; content_length(&headers)];
    if reader.read_exact(&mut body).await.is_err() {
        return;
    }
    let Some(offset) = stream_offset(&first_line) else {
        return;
    };
    let response = stream_response(offset, body, state).await;
    if let Some(response) = response {
        let stream = reader.get_mut();
        let _ = stream.write_all(&response).await;
        let _ = stream.shutdown().await;
    }
}

async fn stream_response(
    offset: u64,
    body: Vec<u8>,
    state: Arc<Mutex<UploadState>>,
) -> Option<Vec<u8>> {
    let mut state = state.lock().await;
    state.stream_offsets.push(offset);
    if offset != state.bytes.len() as u64 {
        return Some(protobuf_http_response(AppendWorkspaceUploadResponse {
            offset: state.bytes.len() as u64,
            complete: false,
        }));
    }
    if state.always_drop_without_progress {
        return None;
    }
    if let Some(commit) = state.dropped_commits.pop_front() {
        let commit = commit.min(body.len());
        state.bytes.extend_from_slice(&body[..commit]);
        return None;
    }
    state.bytes.extend_from_slice(&body);
    Some(protobuf_http_response(AppendWorkspaceUploadResponse {
        offset: state.bytes.len() as u64,
        complete: state.bytes.len() as u64 == state.expected_size,
    }))
}

async fn status_response(request: &str, state: Arc<Mutex<UploadState>>) -> serde_json::Value {
    let node_id = json_string_field(request, "node_id").unwrap_or_default();
    let mut state = state.lock().await;
    state.status_nodes.push(node_id);
    serde_json::json!({
        "type": "RemoteHttpResponse",
        "status": 200,
        "headers": [],
        "body": BeginWorkspaceUploadResponse {
            upload_id: "upload".to_string(),
            offset: state.bytes.len() as u64,
            complete: state.bytes.len() as u64 == state.expected_size,
        }.encode_to_vec(),
    })
}
