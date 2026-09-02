#![cfg(unix)]

use tak_proto::worker_v2::{WorkerIdentity, encode_identity};
use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test]
async fn daemon_onboards_a_direct_v2_invite_through_worker_v2_identity() {
    let root = tempfile::tempdir().expect("temp root");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let identity_url = base_url.clone();
    let node_server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 2048];
            let read = stream.read(&mut request).await.unwrap();
            if request[..read].starts_with(b"PRI * HTTP/2.0") {
                continue;
            }
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /v2/worker/identity HTTP/1.1\r\n"));
            let lower = request.to_ascii_lowercase();
            assert!(
                lower.contains("x-tak-protocol-version: v2\r\n"),
                "{request}"
            );
            assert!(
                lower.contains("authorization: bearer invite-secret\r\n"),
                "{request}"
            );
            assert!(
                lower.contains("x-tak-remote-node: builder-a\r\n"),
                "{request}"
            );
            reply_with_identity(&mut stream, &identity_url).await;
            return;
        }
        panic!("direct onboarding never attempted HTTP/1.1 fallback");
    });
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;
    let invite = direct_invite(&base_url);

    let added = daemon.exchange(&format!(r#"{{"protocol_version":2,"request_id":"add","operation":{{"type":"AddRemote","invite":"{invite}"}}}}"#)).await;

    assert!(added.contains(r#""type":"RemoteAdded""#), "{added}");
    assert!(added.contains(r#""node_id":"builder-a""#), "{added}");
    assert!(added.contains(r#""transport":"direct""#), "{added}");
    assert!(!added.contains("invite-secret"), "{added}");
    node_server.await.unwrap();
}

fn direct_invite(base_url: &str) -> String {
    encode_remote_token(&RemoteTokenPayload {
        version: "v2".into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "Invited Builder".into(),
            base_url: base_url.into(),
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
    .unwrap()
}

async fn reply_with_identity(stream: &mut tokio::net::TcpStream, base_url: &str) {
    let body = encode_identity(&WorkerIdentity {
        protocol_version: 2,
        node_id: "builder-a".into(),
        display_name: "Builder A".into(),
        base_url: base_url.into(),
        pools: vec!["build".into()],
        tags: vec!["linux".into()],
        capabilities: vec!["docker".into()],
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
}
