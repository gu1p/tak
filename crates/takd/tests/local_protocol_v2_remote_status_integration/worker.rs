use prost::Message;
use tak_proto::worker_v2::{
    WorkerResources, WorkerSnapshot, encode_display_payload, encode_snapshot,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::support::takd_tasks::empty_status;

pub(super) async fn serve(listener: tokio::net::TcpListener) {
    let mut replies = 0;
    for _ in 0..4 {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 2048];
        let read = stream.read(&mut request).await.unwrap();
        if request[..read].starts_with(b"PRI * HTTP/2.0") {
            continue;
        }
        let request = String::from_utf8_lossy(&request[..read]);
        assert_protocol_and_credentials(&request);
        let body = response_body(&request);
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
        replies += 1;
        if replies == 2 {
            return;
        }
    }
    panic!("status never attempted HTTP/1.1 fallback");
}

fn assert_protocol_and_credentials(request: &str) {
    let request = request.to_ascii_lowercase();
    assert!(request.contains("x-tak-protocol-version: v2"));
    assert!(request.contains("authorization: bearer status-secret"));
}

fn response_body(request: &str) -> Vec<u8> {
    if request.contains("GET /v2/worker/snapshot") {
        return encode_snapshot(&snapshot()).unwrap();
    }
    assert!(request.contains("GET /v2/worker/status"), "{request}");
    encode_display_payload(&empty_status("builder-a").encode_to_vec()).unwrap()
}

fn snapshot() -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "builder-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(8000, 16000, 8),
        usage: resources(1000, 4000, 2),
        queue_depth: 1,
        cached_content: vec![],
        processes: vec![],
    }
}

fn resources(cpu_millis: u64, memory_bytes: u64, execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis,
        memory_bytes,
        execution_slots,
    }
}
