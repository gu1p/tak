use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_core::v2::RemoteRequirements;
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot, encode_snapshot};
use takd::{PeerManager, TorBroker};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn direct_worker_probe_uses_only_the_authenticated_v2_snapshot_route() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let count = stream.read(&mut request).await.unwrap();
            if request[..count].starts_with(b"PRI * HTTP/2.0") {
                continue;
            }
            answer_snapshot(&mut stream, &request[..count]).await;
            return;
        }
        panic!("worker probe never attempted HTTP/1.1 fallback");
    });
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "worker-a".into(), display_name: "worker-a".into(),
            base_url: format!("http://{address}"), bearer_token: "secret".into(),
            pools: vec!["build".into()], tags: vec!["builder".into()],
            capabilities: vec!["linux".into()], transport: "direct".into(), enabled: true,
        }],
    });
    peers.probe_workers_once(&TorBroker::new()).await;
    server.await.unwrap();
    let candidates = peers.remote_candidates(&RemoteRequirements {
        pool: Some("build".into()), required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()], transport: Some("direct".into()),
    });
    assert_eq!(candidates[0].node_id, "worker-a");
}

async fn answer_snapshot(stream: &mut tokio::net::TcpStream, request: &[u8]) {
    let request = String::from_utf8_lossy(request);
    assert!(request.starts_with("GET /v2/worker/snapshot HTTP/"), "{request}");
    assert_eq!(request.matches("X-Tak-Protocol-Version: v2").count(), 1);
    assert!(request.to_ascii_lowercase().contains("authorization: bearer secret"));
    let body = encode_snapshot(&snapshot()).unwrap();
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(&body).await.unwrap();
}

fn snapshot() -> WorkerSnapshot {
    WorkerSnapshot { protocol_version: 2, node_id: "worker-a".into(), healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources { cpu_millis: 8_000, memory_bytes: 16_000,
            execution_slots: 8 },
        usage: WorkerResources { cpu_millis: 0, memory_bytes: 0, execution_slots: 0 },
        queue_depth: 0, cached_content: vec![], processes: vec![] }
}
