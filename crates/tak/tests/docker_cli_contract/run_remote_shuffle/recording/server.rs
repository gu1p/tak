use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use crate::support::remote_cli::node_info;
use super::http::{read_request_head, write_protobuf_response};
use super::responses::{
    error_response, events_response, status_response, submit_response, success_result,
};

pub(crate) struct RecordingDockerRunNode {
    pub(crate) base_url: String,
    pub(crate) node_id: String,
    handle: Option<thread::JoinHandle<()>>,
    port: u16,
}

impl RecordingDockerRunNode {
    pub(crate) fn spawn(node_id: &str, status_known: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind docker run remote");
        let port = listener.local_addr().expect("remote addr").port();
        let base_url = format!("http://127.0.0.1:{port}");
        let node_id = node_id.to_string();
        let thread_node_id = node_id.clone();
        let thread_base_url = base_url.clone();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept remote request");
                if !serve_request(&mut stream, &thread_node_id, &thread_base_url, status_known) {
                    break;
                }
            }
        });
        Self {
            base_url,
            node_id,
            handle: Some(handle),
            port,
        }
    }
}

impl Drop for RecordingDockerRunNode {
    fn drop(&mut self) {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
            let _ = stream.write_all(
                b"GET /__shutdown HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            );
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn serve_request(
    stream: &mut TcpStream,
    node_id: &str,
    base_url: &str,
    status_known: bool,
) -> bool {
    let request = read_request_head(stream);
    match request_path(&request) {
        "/__shutdown" => {
            write_protobuf_response(stream, "200 OK", &submit_response("shutdown"));
            false
        }
        "/v1/node/info" => {
            write_protobuf_response(stream, "200 OK", &node_info(node_id, base_url, "direct"));
            true
        }
        "/v1/node/status" => {
            if status_known {
                write_protobuf_response(stream, "200 OK", &status_response(node_id, base_url));
            } else {
                write_protobuf_response(stream, "404 Not Found", &error_response("not found"));
            }
            true
        }
        "/v1/tasks/submit" => {
            write_protobuf_response(stream, "200 OK", &submit_response("docker-run-test:1"));
            true
        }
        path if path.contains("/events") => {
            write_protobuf_response(stream, "200 OK", &events_response(node_id));
            true
        }
        path if path.contains("/result") => {
            write_protobuf_response(stream, "200 OK", &success_result(node_id));
            true
        }
        _ => {
            write_protobuf_response(stream, "404 Not Found", &error_response("not found"));
            true
        }
    }
}

fn request_path(request: &str) -> &str {
    request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or_default()
}
