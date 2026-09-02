use tak_proto::worker_v2::{
    WorkerIdentity, WorkerResources, WorkerSnapshot, encode_identity, encode_snapshot,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn spawn(onion: &'static str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dial = listener.local_addr().unwrap().to_string();
    let server = tokio::spawn(async move {
        let mut served_identity = false;
        for _ in 0..6 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 2048];
            let read = stream.read(&mut request).await.unwrap();
            if request[..read].starts_with(b"PRI * HTTP/2.0") {
                continue;
            }
            let request = String::from_utf8_lossy(&request[..read]);
            assert_v2_auth(&request);
            let (body, done) = if request.contains("GET /v2/worker/identity") {
                served_identity = true;
                (identity(onion), false)
            } else if request.contains("GET /v2/worker/snapshot") {
                assert!(served_identity, "snapshot must follow identity onboarding");
                (snapshot(), true)
            } else {
                panic!("unexpected worker request: {request}")
            };
            write_response(&mut stream, &body).await;
            if done {
                return;
            }
        }
        panic!("onboarding and follow-up snapshot did not use HTTP/1.1 fallback");
    });
    (dial, server)
}

fn assert_v2_auth(request: &str) {
    let request = request.to_ascii_lowercase();
    assert!(request.contains("x-tak-protocol-version: v2"));
    assert!(request.contains("authorization: bearer invite-secret"));
}

fn identity(onion: &str) -> Vec<u8> {
    encode_identity(&WorkerIdentity {
        protocol_version: 2,
        node_id: "builder-a".into(),
        display_name: "Builder A".into(),
        base_url: format!("http://{onion}"),
        pools: vec!["build".into()],
        tags: vec!["linux".into()],
        capabilities: vec!["docker".into()],
        transport: "tor".into(),
    })
    .unwrap()
}

fn snapshot() -> Vec<u8> {
    encode_snapshot(&WorkerSnapshot {
        protocol_version: 2,
        node_id: "builder-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(4_000, 8_000, 2),
        usage: resources(0, 0, 0),
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    })
    .unwrap()
}

fn resources(cpu_millis: u64, memory_bytes: u64, execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis,
        memory_bytes,
        execution_slots,
    }
}

async fn write_response(stream: &mut tokio::net::TcpStream, body: &[u8]) {
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
    stream.write_all(body).await.unwrap();
}
