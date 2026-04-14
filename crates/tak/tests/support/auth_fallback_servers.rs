#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use prost::Message;

use super::remote_cli::node_info;

pub fn spawn_auth_rejecting_submit_server(node_id: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind auth listener");
    let base_url = format!("http://{}", listener.local_addr().expect("auth addr"));
    let node_id = node_id.to_string();
    let body = node_info(&node_id, &base_url, "direct").encode_to_vec();
    let handle = thread::spawn(move || {
        respond_with_node_info(&listener, &body);
        respond_with_submit_auth_failure(&listener);
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

fn respond_with_node_info(listener: &TcpListener, body: &[u8]) {
    let (mut stream, _) = listener.accept().expect("accept node info");
    let request = read_request_head(&mut stream);
    assert!(
        request.starts_with("GET /v1/node/info HTTP/1.1\r\n"),
        "unexpected request: {request}"
    );
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write node info head");
    stream.write_all(body).expect("write node info body");
}

fn respond_with_submit_auth_failure(listener: &TcpListener) {
    let (mut stream, _) = listener.accept().expect("accept submit");
    let request = read_request_head(&mut stream);
    assert!(
        request.starts_with("POST /v1/tasks/submit HTTP/1.1\r\n"),
        "unexpected request: {request}"
    );
    write!(
        stream,
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .expect("write submit response");
}

fn read_request_head(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 256];
    loop {
        let read = stream.read(&mut buf).expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            request.truncate(index + 4);
            break;
        }
    }
    String::from_utf8(request).expect("request head utf8")
}
