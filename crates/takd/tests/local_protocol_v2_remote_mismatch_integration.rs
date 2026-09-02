use tak_core::remote_inventory::{RemoteInventory, RemoteRecord, save_remote_inventory_at};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::raw_local_protocol::RawLocalProtocol;

const UPGRADE: &str = "upgrade tak, takd, and workers together";
const ONION: &str = "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[tokio::test]
async fn onboarding_non_v2_worker_returns_a_redacted_protocol_error() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dial = listener.local_addr().unwrap().to_string();
    let worker = tokio::spawn(async move {
        answer_after_h2_fallback(&listener, br#"{"protocol_version":1,"node_id":"old"}"#).await;
    });
    let mut daemon = RawLocalProtocol::start_with_remote_inventory(
        root.path(),
        takd::TorBroker::for_direct_dial(dial),
    )
    .await;
    let invite = tak_proto::encode_tor_invite(&format!("http://{ONION}")).unwrap();
    let response = daemon.exchange(&format!(
        r#"{{"protocol_version":2,"request_id":"add","operation":{{"type":"AddRemote","invite":"{invite}"}}}}"#,
    )).await;

    assert!(
        response.contains(r#""code":"protocol_version_unsupported""#),
        "{response}"
    );
    assert!(
        response.to_ascii_lowercase().contains(UPGRADE),
        "{response}"
    );
    assert!(!response.contains("bearer_token"), "{response}");
    worker.await.unwrap();
}

#[tokio::test]
async fn configured_non_v2_worker_is_reported_with_coordinated_upgrade_guidance() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let path = root.path().join("config/tak/remotes.toml");
    save_remote_inventory_at(
        &path,
        &RemoteInventory {
            version: 1,
            remotes: vec![RemoteRecord {
                node_id: "worker-old".into(),
                display_name: "Worker Old".into(),
                base_url: format!("http://{address}"),
                bearer_token: "secret".into(),
                pools: vec![],
                tags: vec![],
                capabilities: vec![],
                transport: "direct".into(),
                enabled: true,
            }],
        },
    )
    .unwrap();
    let worker = tokio::spawn(async move {
        answer_after_h2_fallback(
            &listener,
            br#"{"protocol_version":1,"node_id":"worker-old"}"#,
        )
        .await;
    });
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;

    let response = daemon.exchange(r#"{"protocol_version":2,"request_id":"status","operation":{"type":"GetRemoteStatus","node_ids":["worker-old"]}}"#).await;

    assert!(response.contains(r#""type":"RemoteStatus""#), "{response}");
    assert!(
        response.to_ascii_lowercase().contains(UPGRADE),
        "{response}"
    );
    assert!(!response.contains("secret"), "{response}");
    worker.await.unwrap();
}

async fn answer_after_h2_fallback(listener: &tokio::net::TcpListener, body: &[u8]) {
    for _ in 0..2 {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 2048];
        let read = stream.read(&mut request).await.unwrap();
        if request[..read].starts_with(b"PRI * HTTP/2.0") {
            continue;
        }
        stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
        return;
    }
    panic!("worker request never used HTTP/1.1 fallback");
}
