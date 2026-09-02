use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{Duration, sleep};

use super::RawLocalProtocol;

impl Drop for RawLocalProtocol {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub(super) async fn exchange(stream: &mut BufReader<UnixStream>, request: &str) -> String {
    stream
        .get_mut()
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    stream
        .get_mut()
        .write_all(b"\n")
        .await
        .expect("write newline");
    stream.get_mut().flush().await.expect("flush request");

    let mut response = String::new();
    stream
        .read_line(&mut response)
        .await
        .expect("read response");
    response
}

pub(super) async fn connect(socket_path: &Path) -> UnixStream {
    let connection_path = super::super::socket_path::bind_path(socket_path);
    for _ in 0..50 {
        if let Ok(stream) = UnixStream::connect(&connection_path).await {
            return stream;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out connecting to {}", socket_path.display());
}
