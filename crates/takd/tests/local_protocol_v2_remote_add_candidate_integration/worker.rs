use tak_proto::worker_v2::{
    WorkerIdentity, WorkerResources, WorkerSnapshot, encode_identity, encode_snapshot,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod invite;
pub(super) use invite::direct_invite;

pub(super) async fn serve(listener: tokio::net::TcpListener, base_url: String) {
    serve_with_snapshot_failures(listener, base_url, 0).await;
}

pub(super) async fn serve_with_transient_snapshot_failure(
    listener: tokio::net::TcpListener,
    base_url: String,
) {
    serve_with_snapshot_failures(listener, base_url, 1).await;
}

async fn serve_with_snapshot_failures(
    listener: tokio::net::TcpListener,
    base_url: String,
    mut remaining_snapshot_failures: usize,
) {
    let mut snapshot_served = false;
    while !snapshot_served {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 4096];
        let read = stream.read(&mut request).await.unwrap();
        if request[..read].starts_with(b"PRI * HTTP/2.0") {
            continue;
        }
        let request = String::from_utf8_lossy(&request[..read]);
        let (status, body) = if request.starts_with("GET /v2/worker/identity ") {
            (200, encode_identity(&identity(&base_url)).unwrap())
        } else {
            assert!(request.starts_with("GET /v2/worker/snapshot "), "{request}");
            if remaining_snapshot_failures > 0 {
                remaining_snapshot_failures -= 1;
                (500, b"snapshot_unavailable".to_vec())
            } else {
                snapshot_served = true;
                (200, encode_snapshot(&snapshot()).unwrap())
            }
        };
        let head = format!(
            "HTTP/1.1 {status} Test Response\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
    }
}

fn identity(base_url: &str) -> WorkerIdentity {
    WorkerIdentity {
        protocol_version: 2,
        node_id: "builder-a".into(),
        display_name: "Builder A".into(),
        base_url: base_url.into(),
        pools: vec!["build".into()],
        tags: vec!["linux".into()],
        capabilities: vec!["docker".into()],
        transport: "direct".into(),
    }
}

fn snapshot() -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "builder-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources {
            cpu_millis: 4_000,
            memory_bytes: 8_000,
            execution_slots: 2,
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
