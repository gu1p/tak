use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use super::super::{State, responses};

pub(super) async fn serve(
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
