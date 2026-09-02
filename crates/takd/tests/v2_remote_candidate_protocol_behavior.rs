#[path = "v2_remote_candidate_protocol_fixture.rs"]
mod fixture;

use fixture::{exchange, record, snapshot, wait_for};
use tak_core::remote_inventory::RemoteInventory;
use tak_core::v2::RemoteRequirements;
use tak_proto::local_daemon::v2::{Operation, Request, Response, decode_response, encode_request};
use takd::{PeerManager, TorBroker, new_shared_manager_with_db};

#[tokio::test(flavor = "multi_thread")]
async fn local_v2_protocol_returns_only_matching_connected_v2_candidates() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![record("direct-v2", "direct"), record("tor-v2", "tor")],
    });
    for node in ["direct-v2", "tor-v2"] {
        peers.mark_worker_snapshot(node, snapshot(node));
    }
    let manager = new_shared_manager_with_db(db.clone()).unwrap();
    let remote_inventory = temp.path().join("missing-remotes.toml");
    let serve_socket = socket.clone();
    let mut server = tokio::spawn(async move {
        takd::run_server_with_broker_peers_and_remote_inventory(
            &serve_socket,
            manager,
            TorBroker::new(),
            peers,
            remote_inventory,
        )
        .await
    });
    tokio::select! {
        () = wait_for(|| socket.exists()) => {}
        result = &mut server => panic!("server stopped before socket was ready: {result:?}"),
    }
    let request = Request {
        request_id: "remote-candidates".into(),
        operation: Operation::ResolveRemoteCandidates {
            requirements: RemoteRequirements {
                pool: Some("build".into()),
                required_tags: vec!["builder".into()],
                required_capabilities: vec!["linux".into()],
                transport: Some("direct".into()),
            },
        },
    };
    let raw = exchange(&socket, &encode_request(&request).unwrap()).await;
    let Response::RemoteCandidates { candidates, .. } =
        decode_response(raw.trim().as_bytes(), "remote-candidates").unwrap()
    else {
        panic!("expected candidate response")
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].node_id, "direct-v2");
    server.abort();
}
