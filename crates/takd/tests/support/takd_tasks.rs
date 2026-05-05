use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Command as StdCommand, Output};
use std::thread;

use prost::Message;
use tak_proto::NodeStatusResponse;

use crate::support::takd_bin;

pub fn run_takd_tasks(config_root: &Path, state_root: &Path) -> Output {
    StdCommand::new(takd_bin())
        .args([
            "tasks",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd tasks")
}

pub fn spawn_status_socket(
    state_root: &Path,
    bearer_token: &str,
    status: NodeStatusResponse,
) -> thread::JoinHandle<()> {
    std::fs::create_dir_all(state_root).expect("create state root");
    let socket_path = state_root.join("agent-control.sock");
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(socket_path).expect("bind fake control socket");
    let bearer_token = bearer_token.to_string();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept control request");
        let request = read_http_head(&mut stream);
        assert!(
            request.contains(&format!("Authorization: Bearer {bearer_token}\r\n")),
            "missing bearer auth:\n{request}"
        );
        let body = status.encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
