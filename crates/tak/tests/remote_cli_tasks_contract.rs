use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use support::remote_cli::read_request;
use support::remote_status::write_inventory;
use tak_proto::{ListTaskAttemptsResponse, TaskAttemptSummary};

#[test]
fn remote_tasks_lists_attempts_from_selected_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tasks server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    write_inventory(&config_root, "builder-a", &base_url);

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept tasks request");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with("GET /v1/tasks?state=all&limit=50 HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let body = ListTaskAttemptsResponse {
            attempts: vec![TaskAttemptSummary {
                task_run_id: "task-run-remote-1".into(),
                attempt: 1,
                task_label: "//apps/web:build".into(),
                node_id: "builder-a".into(),
                state: "completed".into(),
                created_at_ms: 10,
                finished_at_ms: Some(20),
                execution_label: Some("check.build".into()),
            }],
        }
        .encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "tasks", "--node", "builder-a"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote tasks");

    assert!(
        output.status.success(),
        "tak remote tasks should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Remote Tasks"), "missing header:\n{stdout}");
    assert!(
        stdout.contains("task_run_id=task-run-remote-1"),
        "missing task run id:\n{stdout}"
    );
    assert!(
        stdout.contains("task_label=check.build"),
        "missing execution label:\n{stdout}"
    );
    assert!(
        stdout.contains("state=completed"),
        "missing state:\n{stdout}"
    );
    server.join().expect("tasks server should exit");
}
