use std::time::Duration;

use tak_core::remote_inventory::RemoteRecord;
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub(super) fn record(node: &str, transport: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node.into(),
        display_name: node.into(),
        base_url: if transport == "direct" {
            "http://127.0.0.1:1".into()
        } else {
            format!("http://{node}.onion")
        },
        bearer_token: "secret".into(),
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: transport.into(),
        enabled: true,
    }
}

pub(super) fn snapshot(node: &str) -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: node.into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources {
            cpu_millis: 8_000,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            execution_slots: 8,
        },
        usage: WorkerResources {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: 0,
        },
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}

pub(super) async fn exchange(socket: &std::path::Path, frame: &str) -> String {
    let connection_path = crate::support::socket_path::bind_path(socket);
    let mut stream = tokio::net::UnixStream::connect(connection_path)
        .await
        .unwrap();
    stream.write_all(frame.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).await.unwrap();
    line
}

pub(super) async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
