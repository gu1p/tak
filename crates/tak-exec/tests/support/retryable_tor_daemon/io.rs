use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

use super::State;
use super::responses;

pub(super) async fn serve(listener: UnixListener, state: Arc<Mutex<State>>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(serve_connection(stream, Arc::clone(&state)));
    }
}

async fn serve_connection(stream: UnixStream, state: Arc<Mutex<State>>) {
    let mut reader = BufReader::new(stream);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).await.unwrap_or(0) == 0 {
        return;
    }
    if first_line.starts_with('{') {
        serve_json(reader, first_line, state).await;
        return;
    }
    serve_stream(reader, first_line, state).await;
}

async fn serve_json(mut reader: BufReader<UnixStream>, line: String, state: Arc<Mutex<State>>) {
    let value = serde_json::from_str::<serde_json::Value>(&line).expect("daemon request json");
    let response = match value.get("type").and_then(|value| value.as_str()) {
        Some("PeersEligible") => peers(state).await,
        Some("ForwardRemoteHttp") => upload_status(&value, state).await,
        Some("PlaceRemote") => place_remote(&value, state).await,
        Some("StreamTaskEvents") => responses::events(),
        Some("GetTaskResult") => responses::result(),
        _ => responses::error("unexpected daemon request"),
    };
    let stream = reader.get_mut();
    let _ = stream.write_all(response.to_string().as_bytes()).await;
    let _ = stream.write_all(b"\n").await;
}

async fn peers(state: Arc<Mutex<State>>) -> serde_json::Value {
    let mut state = state.lock().await;
    state.peer_requests += 1;
    if state.non_retryable_peers {
        return responses::classified_error(
            "No known remote worker satisfies this task's requirements.",
            "resource_requirements_exceed_worker_capacity",
            false,
        );
    }
    responses::peers()
}

async fn upload_status(request: &serde_json::Value, state: Arc<Mutex<State>>) -> serde_json::Value {
    let path = request
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let upload_id = path
        .trim_start_matches("/v2/workspaces/uploads/")
        .to_string();
    let state = state.lock().await;
    responses::upload_status(&upload_id, state.committed, state.committed == state.size)
}

async fn place_remote(request: &serde_json::Value, state: Arc<Mutex<State>>) -> serde_json::Value {
    if let Some(attempt) = request.get("attempt").and_then(|value| value.as_u64()) {
        state.lock().await.submit_attempts.push(attempt as u32);
    }
    responses::placed()
}

async fn serve_stream(
    mut reader: BufReader<UnixStream>,
    first_line: String,
    state: Arc<Mutex<State>>,
) {
    let headers = responses::stream::read_headers(&mut reader).await;
    let content_len = responses::stream::content_length(&headers);
    let mut body = vec![0_u8; content_len];
    if reader.read_exact(&mut body).await.is_err() {
        return;
    }
    let should_respond = {
        let mut state = state.lock().await;
        responses::stream::record_stream(&first_line, &headers, &mut state)
    };
    if should_respond {
        let stream = reader.get_mut();
        let state = state.lock().await;
        let response = responses::stream::stream_response(&state);
        let _ = stream.write_all(&response).await;
    }
}
