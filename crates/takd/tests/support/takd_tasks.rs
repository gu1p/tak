use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread;

use prost::Message;
use tak_proto::NodeStatusResponse;

#[path = "takd_tasks/command.rs"]
mod command;

pub use command::run_takd_tasks;

pub fn spawn_status_socket(
    state_root: &Path,
    bearer_token: &str,
    status: NodeStatusResponse,
) -> thread::JoinHandle<()> {
    std::fs::create_dir_all(state_root).expect("create state root");
    let socket_path = state_root.join("agent-control.sock");
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(super::socket_path::bind_path(&socket_path))
        .expect("bind fake control socket");
    let bearer_token = bearer_token.to_string();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept control request");
        let request = read_http_head(&mut stream);
        assert!(
            request.contains(&format!("Authorization: Bearer {bearer_token}\r\n")),
            "missing bearer auth:\n{request}"
        );
        assert!(
            request.starts_with("GET /v2/worker/status HTTP/1.1\r\n"),
            "unexpected control status route:\n{request}"
        );
        assert!(
            request.contains("X-Tak-Protocol-Version: v2\r\n"),
            "missing worker v2 protocol header:\n{request}"
        );
        let body = tak_proto::worker_v2::encode_display_payload(&status.encode_to_vec())
            .expect("encode worker v2 status envelope");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write control response head");
        stream
            .write_all(&body)
            .expect("write control response body");
    })
}

pub fn empty_status(node_id: &str) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(tak_proto::NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: "http://127.0.0.1:0".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: 1,
        cpu: None,
        memory: None,
        storage: None,
        allocated_needs: Vec::new(),
        active_jobs: Vec::new(),
        image_cache: None,
        queued_jobs: Vec::new(),
        resource_envelope: None,
        resource_pressure: None,
    }
}

fn read_http_head(stream: &mut std::os::unix::net::UnixStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 128];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("read control request");
        assert_ne!(read, 0, "request ended before headers");
        bytes.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(bytes).expect("control request utf8")
}
