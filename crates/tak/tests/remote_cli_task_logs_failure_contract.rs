use crate::support;
use std::{io::Write, net::TcpListener, process::Command as StdCommand, thread};

use prost::Message;
use support::remote_cli::read_request;
use support::remote_status::write_inventory;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

#[test]
fn remote_task_logs_prints_terminal_failure_message_to_stderr() {
    assert_remote_task_log_stderr(
        terminal_event(
            "TASK_FAILED",
            Some("worker exited before returning a result"),
            Some(1),
        ),
        "worker exited before returning a result\n",
    );
}

#[test]
fn remote_task_logs_prints_terminal_failure_exit_code_to_stderr() {
    assert_remote_task_log_stderr(
        terminal_event("TASK_FAILED", None, Some(137)),
        "remote task failed with exit code 137\n",
    );
}

#[test]
fn remote_task_logs_prints_terminal_cancelled_exit_code_to_stderr() {
    assert_remote_task_log_stderr(
        terminal_event("TASK_CANCELLED", None, Some(137)),
        "remote task cancelled with exit code 137\n",
    );
}

fn terminal_event(kind: &str, message: Option<&str>, exit_code: Option<i32>) -> RemoteEvent {
    RemoteEvent {
        seq: 1,
        kind: kind.into(),
        timestamp_ms: 12,
        success: Some(false),
        exit_code,
        message: message.map(str::to_string),
        chunk: None,
        chunk_bytes: Vec::new(),
    }
}

fn assert_remote_task_log_stderr(event: RemoteEvent, expected_stderr: &str) {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind task logs server");
    let addr = listener.local_addr().expect("listener addr");
    write_inventory(&config_root, "builder-a", &format!("http://{addr}"));

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept task logs request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/tasks/task-run-failed/events?after_seq=0 HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = PollTaskEventsResponse {
            events: vec![event],
            done: true,
        }
        .encode_to_vec();
        write_response(&mut stream, &body);
    });

    let output = StdCommand::new(support::tak_bin())
        .args([
            "remote",
            "task",
            "logs",
            "--node",
            "builder-a",
            "task-run-failed",
        ])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote task logs");

    assert!(
        output.status.success(),
        "tak remote task logs should succeed"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected_stderr);
    server.join().expect("task logs server should exit");
}

fn write_response(stream: &mut impl Write, body: &[u8]) {
    write!(stream, "HTTP/1.1 200 OK\r\n").expect("write status");
    write!(stream, "Content-Type: application/x-protobuf\r\n").expect("write content type");
    write!(stream, "Content-Length: {}\r\n", body.len()).expect("write content length");
    write!(stream, "Connection: close\r\n\r\n").expect("write connection");
    stream.write_all(body).expect("write response body");
}
