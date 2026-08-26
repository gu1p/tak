use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

use super::State;
mod json;
mod stream;

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
        json::serve(reader, first_line, state).await;
        return;
    }
    stream::serve(reader, first_line, state).await;
}
