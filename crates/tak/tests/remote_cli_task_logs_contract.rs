use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use support::remote_cli::read_request;
use support::remote_status::write_inventory;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

#[test]
fn remote_task_logs_streams_persisted_stdout_and_stderr() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind task logs server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept task logs request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/tasks/task-run-remote-1/events?after_seq=0 HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = PollTaskEventsResponse {
            events: vec![
                RemoteEvent {
                    seq: 1,
                    kind: "TASK_STDOUT_CHUNK".into(),
                    timestamp_ms: 10,
                    success: None,
                    exit_code: None,
                    message: None,
                    chunk: Some("remote stdout\n".into()),
                    chunk_bytes: b"remote stdout\n".to_vec(),
                },
                RemoteEvent {
                    seq: 2,
                    kind: "TASK_STDERR_CHUNK".into(),
                    timestamp_ms: 11,
                    success: None,
                    exit_code: None,
                    message: None,
                    chunk: Some("remote stderr\n".into()),
                    chunk_bytes: b"remote stderr\n".to_vec(),
                },
                RemoteEvent {
                    seq: 3,
                    kind: "TASK_COMPLETED".into(),
                    timestamp_ms: 12,
                    success: Some(true),
                    exit_code: Some(0),
                    message: None,
                    chunk: None,
                    chunk_bytes: Vec::new(),
                },
            ],
            done: true,
        }
        .encode_to_vec();
        write_response_head(&mut stream, body.len());
        stream.write_all(&body).expect("write response body");
    });

    let output = StdCommand::new(support::tak_bin())
        .args([
            "remote",
            "task",
            "logs",
            "--node",
            "builder-a",
            "task-run-remote-1",
        ])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote task logs");

    assert!(
        output.status.success(),
        "tak remote task logs should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "remote stdout\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "remote stderr\n");
    server.join().expect("task logs server should exit");
}

fn write_response_head(stream: &mut impl Write, content_len: usize) {
    write!(stream, "HTTP/1.1 200 OK\r\n").expect("write status");
    write!(stream, "Content-Type: application/x-protobuf\r\n").expect("write content type");
    write!(stream, "Content-Length: {content_len}\r\n").expect("write content length");
    write!(stream, "Connection: close\r\n\r\n").expect("write connection");
}
