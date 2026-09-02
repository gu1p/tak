#![cfg(unix)]

use tak_proto::worker_v2::{WorkerIdentity, encode_identity};
use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test]
async fn direct_onboarding_rejects_an_identity_that_does_not_match_the_invite() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let served_url = base_url.clone();
    let worker = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).await.unwrap();
            if request[..read].starts_with(b"PRI * HTTP/2.0") {
                continue;
            }
            let body = encode_identity(&WorkerIdentity {
                protocol_version: 2,
                node_id: "different-worker".into(),
                display_name: "Impostor".into(),
                base_url: served_url,
                pools: vec![],
                tags: vec![],
                capabilities: vec![],
                transport: "direct".into(),
            })
            .unwrap();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(&body).await.unwrap();
            return;
        }
    });
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;
    let invite = encode_remote_token(&RemoteTokenPayload {
        version: "v2".into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "Builder".into(),
            base_url,
            healthy: true,
            pools: vec![],
            tags: vec![],
            capabilities: vec![],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        bearer_token: "invite-secret".into(),
    })
    .unwrap();

    let response = daemon.exchange(&format!(r#"{{"protocol_version":2,"request_id":"add","operation":{{"type":"AddRemote","invite":"{invite}"}}}}"#)).await;

    assert!(response.contains(r#""type":"Error""#), "{response}");
    assert!(!response.contains(r#""type":"RemoteAdded""#), "{response}");
    worker.await.unwrap();
}
