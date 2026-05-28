#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use prost::Message;
use tak_proto::NodeInfo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct RecordingRemote {
    pub addr: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl RecordingRemote {
    pub async fn spawn(node_id: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        spawn_accept_loop(listener, node_id.to_string(), Arc::clone(&requests));
        Self { addr, requests }
    }

    pub fn request_count(&self) -> usize {
        self.requests.lock().expect("request lock").len()
    }

    pub fn single_request(&self) -> String {
        self.requests
            .lock()
            .expect("request lock")
            .first()
            .cloned()
            .expect("remote request")
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("request lock").clone()
    }
}

fn spawn_accept_loop(listener: TcpListener, node_id: String, requests: Arc<Mutex<Vec<String>>>) {
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            let request = read_request(&mut stream).await;
            requests.lock().expect("request lock").push(request);
            let body = node_info(&node_id).encode_to_vec();
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).await.expect("write head");
            stream.write_all(&body).await.expect("write body");
        }
    });
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 512];
    loop {
        let read = stream.read(&mut chunk).await.expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&request).to_string()
}

fn node_info(node_id: &str) -> NodeInfo {
    NodeInfo {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.onion"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "tor".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
}
