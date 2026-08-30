use std::path::Path;

use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{Duration, sleep, timeout};

pub struct RawLocalProtocol {
    _temp: Option<TempDir>,
    server: tokio::task::JoinHandle<anyhow::Result<()>>,
    stream: BufReader<UnixStream>,
}

impl RawLocalProtocol {
    pub async fn start() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let socket_path = temp.path().join("run/takd.sock");
        let server = super::protocol_server::spawn_protocol_server(
            temp.path().join("state/takd.sqlite"),
            socket_path.clone(),
        );
        let stream = connect(&socket_path).await;
        Self {
            _temp: Some(temp),
            server,
            stream: BufReader::new(stream),
        }
    }

    pub async fn start_in(root: &Path) -> Self {
        let socket_path = root.join("run/takd.sock");
        let server = super::protocol_server::spawn_protocol_server(
            root.join("state/takd.sqlite"),
            socket_path.clone(),
        );
        let stream = connect(&socket_path).await;
        Self {
            _temp: None,
            server,
            stream: BufReader::new(stream),
        }
    }

    pub async fn start_with_manager(manager: takd::SharedLeaseManager) -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let socket_path = temp.path().join("run/takd.sock");
        let server_socket = socket_path.clone();
        let server = tokio::spawn(async move {
            takd::run_server_with_broker_and_peers(
                &server_socket,
                manager,
                takd::TorBroker::new(),
                takd::PeerManager::default(),
            )
            .await
        });
        let stream = connect(&socket_path).await;
        Self {
            _temp: Some(temp),
            server,
            stream: BufReader::new(stream),
        }
    }

    pub async fn exchange(&mut self, request: &str) -> String {
        timeout(Duration::from_secs(5), exchange(&mut self.stream, request))
            .await
            .expect("protocol exchange timed out")
    }
}

async fn exchange(stream: &mut BufReader<UnixStream>, request: &str) -> String {
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

impl Drop for RawLocalProtocol {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn connect(socket_path: &Path) -> UnixStream {
    for _ in 0..50 {
        if let Ok(stream) = UnixStream::connect(socket_path).await {
            return stream;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out connecting to {}", socket_path.display());
}
