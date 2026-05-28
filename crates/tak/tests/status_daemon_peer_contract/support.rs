use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::thread::{self, JoinHandle};

use crate::support;
use support::remote_cli::read_request;
use support::remote_status::status_payload_for;

pub(super) fn spawn_peer_daemon(
    socket_path: &std::path::Path,
    expected_requests: usize,
) -> thread::JoinHandle<()> {
    spawn_peer_daemon_with_state(socket_path, expected_requests, "connected", None)
}

pub(super) fn spawn_peer_daemon_with_state(
    socket_path: &std::path::Path,
    expected_requests: usize,
    state: &'static str,
    error: Option<&'static str>,
) -> thread::JoinHandle<()> {
    let socket_path = socket_path.to_path_buf();
    thread::spawn(move || {
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).expect("socket parent");
        }
        let listener = UnixListener::bind(&socket_path).expect("bind fake takd socket");
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().expect("accept daemon request");
            let mut request = String::new();
            BufReader::new(stream.try_clone().expect("clone stream"))
                .read_line(&mut request)
                .expect("read daemon request");
            let response = if request.contains(r#""type":"Status""#) {
                status_response().to_string()
            } else if request.contains(r#""type":"PeersList""#) {
                peers_response(state, error)
            } else {
                panic!("unexpected daemon request: {request}");
            };
            writeln!(stream, "{response}").expect("write daemon response");
        }
    })
}

fn status_response() -> &'static str {
    r#"{"type":"StatusSnapshot","request_id":"tak-status","status":{"active_leases":0,"pending_requests":0,"usage":[]}}"#
}

fn peers_response(state: &str, error: Option<&str>) -> String {
    let last_error = error
        .map(|value| format!(r#""{value}""#))
        .unwrap_or_else(|| "null".to_string());
    format!(
        r#"{{"type":"PeersSnapshot","request_id":"peers","peers":[{{"node_id":"builder-a","display_name":"Builder A","transport":"tor","endpoint":"http://builder-a.onion","state":"{state}","last_heartbeat_ms":1734000000000,"last_successful_connection_ms":1734000000000,"last_error_summary":{last_error},"active_job_count":1,"queue_depth":0,"resource_summary":"cpu=12.5% ram=2.0GiB/8.0GiB storage=3.0GiB/10.0GiB","protocol_version":"takd.v1","heartbeat_rtt_ms":37,"reconnect_attempts":0,"pools":["build"],"tags":["linux"],"capabilities":["docker"]}}]}}"#
    )
}

pub(super) fn spawn_direct_status_server(node_id: &'static str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind direct status server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let server_base_url = base_url.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept direct status request");
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /v1/node/status HTTP/1.1\r\n"));
        let body = status_payload_for(node_id, &server_base_url, "direct", false);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });
    (base_url, server)
}
