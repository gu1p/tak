use std::path::Path;

use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use takd::{PeerManager, TorBroker, run_server_with_broker_and_peers};

use crate::support;

pub(super) struct AuthServer {
    pub(super) peers: PeerManager,
    server: tokio::task::JoinHandle<()>,
}

impl AuthServer {
    pub(super) fn abort(self) {
        self.server.abort();
    }
}

pub(super) async fn spawn_auth_server(socket_path: &Path, status: u16) -> AuthServer {
    let remote = support::http2_remote::Http2Remote::spawn_status(status, Vec::new()).await;
    let peers = tor_peer_manager();
    let broker = TorBroker::for_test_dial_addr(remote.addr);
    // Mirror production: heartbeats probe the peer, so an auth-rejecting node is
    // marked AuthFailed before any submit is placed on it.
    peers.spawn_heartbeat_loop(broker.clone());
    let server_peers = peers.clone();
    let server_socket_path = socket_path.to_path_buf();
    let server = tokio::spawn(async move {
        let _ = run_server_with_broker_and_peers(
            &server_socket_path,
            takd::new_shared_manager(),
            broker,
            server_peers,
        )
        .await;
    });
    AuthServer { peers, server }
}

pub(super) async fn wait_for_peer_state(peers: &PeerManager, state: takd::PeerState) {
    for _ in 0..200 {
        if peers
            .snapshots()
            .first()
            .is_some_and(|peer| peer.state == state)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for peer to reach {state:?}");
}

fn tor_peer_manager() -> PeerManager {
    PeerManager::from_inventory(RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "builder-auth".into(),
            display_name: "builder-auth".into(),
            base_url: "http://builder-auth.onion".into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            enabled: true,
        }],
    })
}
