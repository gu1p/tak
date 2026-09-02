use std::path::Path;

use tempfile::TempDir;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::time::{Duration, timeout};

#[path = "raw_local_protocol/io.rs"]
mod io;

use io::{connect, exchange};

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
        let server_socket = super::socket_path::bind_path(&socket_path);
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

    pub async fn start_with_remote_inventory(root: &Path, broker: takd::TorBroker) -> Self {
        let socket_path = root.join("run/takd.sock");
        let db_path = root.join("state/takd.sqlite");
        let inventory_path = root.join("config/tak/remotes.toml");
        let manager = takd::new_shared_manager_with_db(db_path).expect("manager");
        let server_socket = super::socket_path::bind_path(&socket_path);
        let server = tokio::spawn(async move {
            takd::run_server_with_broker_peers_and_remote_inventory(
                &server_socket,
                manager,
                broker,
                takd::PeerManager::default(),
                inventory_path,
            )
            .await
        });
        let stream = connect(&socket_path).await;
        Self {
            _temp: None,
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
