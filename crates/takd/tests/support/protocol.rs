#![allow(dead_code)]

use std::net::TcpListener as StdTcpListener;
use std::path::Path;

use tak_core::model::Scope;
use takd::{AcquireLeaseRequest, ClientInfo, NeedRequest, Request, Response, TaskInfo};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{Duration, sleep};

pub async fn send_request(socket_path: &Path, request: &Request) -> Response {
    for _ in 0..50 {
        if let Ok(stream) = UnixStream::connect(socket_path).await {
            return exchange(stream, request).await;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out connecting to socket {}", socket_path.display());
}

async fn exchange(stream: UnixStream, request: &Request) -> Response {
    let mut stream = stream;
    let encoded = serde_json::to_string(request).expect("encode request");
    stream
        .write_all(encoded.as_bytes())
        .await
        .expect("write request");
    stream.write_all(b"\n").await.expect("write newline");
    stream.shutdown().await.expect("shutdown write half");

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await.expect("read response");
    serde_json::from_str(line.trim_end()).expect("decode response")
}

pub fn acquire_request(request_id: &str) -> AcquireLeaseRequest {
    AcquireLeaseRequest {
        request_id: request_id.to_string(),
        client: ClientInfo {
            user: "alice".into(),
            pid: 7,
            session_id: "sess-1".into(),
        },
        task: TaskInfo {
            label: "//apps/web:build".into(),
            attempt: 1,
        },
        needs: vec![NeedRequest {
            name: "cpu".into(),
            scope: Scope::Machine,
            scope_key: None,
            slots: 1.0,
        }],
        ttl_ms: 30_000,
    }
}

pub fn free_bind_addr() -> String {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind free port");
    let addr = listener.local_addr().expect("local addr").to_string();
    drop(listener);
    addr
}
