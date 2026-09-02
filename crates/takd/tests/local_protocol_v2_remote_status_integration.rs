use tak_core::remote_inventory::{RemoteInventory, RemoteRecord, save_remote_inventory_at};

use crate::support::raw_local_protocol::RawLocalProtocol;

#[path = "local_protocol_v2_remote_status_integration/worker.rs"]
mod worker;

#[tokio::test]
async fn daemon_uses_stored_credentials_for_status_without_returning_them() {
    let root = tempfile::tempdir().expect("temp root");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let inventory_path = root.path().join("config/tak/remotes.toml");
    save_remote_inventory_at(
        &inventory_path,
        &RemoteInventory {
            version: 1,
            remotes: vec![RemoteRecord {
                node_id: "builder-a".into(),
                display_name: "Builder A".into(),
                base_url: format!("http://{address}"),
                bearer_token: "status-secret".into(),
                pools: vec![],
                tags: vec![],
                capabilities: vec![],
                transport: "direct".into(),
                enabled: true,
            }],
        },
    )
    .unwrap();
    let remote = tokio::spawn(worker::serve(listener));
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;

    let response = daemon.exchange(r#"{"protocol_version":2,"request_id":"status","operation":{"type":"GetRemoteStatus","node_ids":["builder-a"]}}"#).await;

    assert!(response.contains(r#""type":"RemoteStatus""#), "{response}");
    assert!(
        response.contains(r#""snapshot":{"protocol_version":2"#),
        "{response}"
    );
    assert!(response.contains("detail_base64"), "{response}");
    assert!(!response.contains("status_base64"));
    assert!(!response.contains("status-secret"));
    assert!(!response.contains("bearer_token"));
    remote.await.unwrap();
}
