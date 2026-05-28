use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::task::JoinHandle;

pub async fn spawn_broker_error(
    socket_path: &Path,
    status: &'static str,
    code: &'static str,
    body: &'static str,
) -> JoinHandle<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).expect("broker socket parent");
    }
    let listener = UnixListener::bind(socket_path).expect("bind fake broker");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept broker request");
        let mut request = [0_u8; 1024];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read broker request");
        let response = format!(
            "HTTP/1.1 {status}\r\nX-Tak-Broker-Error: {code}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write broker error");
    })
}
