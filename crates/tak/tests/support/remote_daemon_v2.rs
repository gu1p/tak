#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use serde_json::Value;

pub struct FakeRemoteDaemon {
    socket: PathBuf,
    thread: JoinHandle<Vec<Value>>,
}

impl FakeRemoteDaemon {
    pub fn spawn(root: &Path, responses: Vec<Value>) -> Self {
        let socket = root.join("takd.sock");
        let bind_path = super::unix_socket_bind_path::short_bind_path(&socket);
        let listener = UnixListener::bind(bind_path).expect("bind fake remote daemon");
        let thread = thread::spawn(move || {
            responses
                .into_iter()
                .map(|mut response| {
                    let (mut stream, _) = listener.accept().expect("accept daemon request");
                    let mut line = String::new();
                    BufReader::new(stream.try_clone().expect("clone daemon stream"))
                        .read_line(&mut line)
                        .expect("read daemon request");
                    let request: Value =
                        serde_json::from_str(&line).expect("decode daemon request");
                    response["protocol_version"] = serde_json::json!(2);
                    response["request_id"] = request["request_id"].clone();
                    writeln!(stream, "{}", serde_json::to_string(&response).unwrap())
                        .expect("write daemon response");
                    request
                })
                .collect()
        });
        Self { socket, thread }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub fn finish(self) -> Vec<Value> {
        self.thread.join().expect("fake remote daemon exits")
    }
}

pub fn remote(node_id: &str) -> Value {
    serde_json::json!({
        "node_id": node_id,
        "display_name": node_id,
        "base_url": format!("http://{node_id}.onion"),
        "pools": ["build"],
        "tags": ["linux"],
        "capabilities": ["docker", "arch:arm64", "os:linux"],
        "transport": "tor",
        "enabled": true
    })
}

pub fn healthy_status(node_id: &str) -> Value {
    serde_json::json!({
        "remote": remote(node_id),
        "snapshot": {
            "protocol_version": 2,
            "node_id": node_id,
            "healthy": true,
            "sampled_at_ms": 1,
            "capacity": {"cpu_millis": 8000, "memory_bytes": 16000, "execution_slots": 8},
            "usage": {"cpu_millis": 1000, "memory_bytes": 4000, "execution_slots": 2},
            "queue_depth": 1,
            "cached_content": [],
            "processes": []
        },
        "detail_base64": null,
        "error": null,
        "peer": null
    })
}
