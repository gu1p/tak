#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{CpuUsage, MemoryUsage, NodeStatusResponse};

use super::remote_cli::node_info;

pub fn spawn_auth_rejecting_submit_server(node_id: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind auth listener");
    let base_url = format!("http://{}", listener.local_addr().expect("auth addr"));
    let node_id = node_id.to_string();
    let node = node_info(&node_id, &base_url, "direct");
    let body = node.encode_to_vec();
    let status_body = NodeStatusResponse {
        node: Some(node),
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 8 * 1024 * 1024 * 1024,
        }),
        storage: None,
        allocated_needs: Vec::new(),
        active_jobs: Vec::new(),
        image_cache: None,
        queued_jobs: Vec::new(),
    }
    .encode_to_vec();
    let handle = thread::spawn(move || {
        respond_with_node_info(&listener, &body);
        respond_with_optional_status_then_submit_auth_failure(&listener, &status_body);
    });
    (base_url, handle)
}

pub fn spawn_timeout_node_info_server(node_id: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind timeout listener");
    let base_url = format!("http://{}", listener.local_addr().expect("timeout addr"));
    let node_id = node_id.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept timeout probe");
        let request = read_request_head(&mut stream);
        assert!(
            request.starts_with("GET /v1/node/info HTTP/1.1\r\n"),
            "unexpected request for {node_id}: {request}"
        );
        thread::sleep(Duration::from_millis(750));
    });
    (base_url, handle)
}

include!("auth_fallback_servers/request_handlers.rs");
