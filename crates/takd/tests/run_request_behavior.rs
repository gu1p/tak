//! Integration behavior for daemon `RunTasks` request handling.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use takd::{Request, Response, RunTasksRequest, new_shared_manager, run_server};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

fn write_tasks(root: &Path, body: &str) {
    fs::create_dir_all(root.join("apps/web")).expect("mkdir");
    fs::write(root.join("apps/web/TASKS.py"), body).expect("write tasks");
}

#[tokio::test]
async fn run_tasks_request_streams_started_result_and_completed() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
SPEC = module_spec(tasks=[
  task("hello", steps=[cmd("sh", "-c", "echo daemon-run > run.log")]),
])
SPEC
"#,
    );

    let socket_path = temp.path().join("takd.sock");
    let manager = new_shared_manager();
    let server_socket = socket_path.clone();
    let server_manager = Arc::clone(&manager);
    let server = tokio::spawn(async move {
        run_server(&server_socket, server_manager)
            .await
            .expect("daemon server should run")
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(socket_path.exists(), "daemon socket should be ready");

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("connect daemon socket");
    let request = Request::RunTasks(RunTasksRequest {
        request_id: "run-req-1".to_string(),
        workspace_root: temp.path().display().to_string(),
        labels: vec!["apps/web:hello".to_string()],
        jobs: 1,
        keep_going: false,
        lease_socket: None,
        lease_ttl_ms: 30_000,
        lease_poll_interval_ms: 200,
        session_id: Some("session-run".to_string()),
        user: Some("alice".to_string()),
    });
    let encoded = serde_json::to_string(&request).expect("serialize request");
    stream
        .write_all(format!("{encoded}\n").as_bytes())
        .await
        .expect("send run request");
    stream.flush().await.expect("flush run request");

    let (reader_half, _) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut lines = Vec::new();
    while lines.len() < 3 {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .expect("read response line");
        if read == 0 {
            break;
        }
        lines.push(line);
    }
    assert_eq!(lines.len(), 3, "run request should stream three responses");

    let started: Response = serde_json::from_str(lines[0].trim_end()).expect("decode started");
    assert!(matches!(started, Response::RunStarted { .. }));

    let task_result: Response =
        serde_json::from_str(lines[1].trim_end()).expect("decode task result");
    assert!(matches!(
        task_result,
        Response::RunTaskResult {
            success: true,
            label,
            ..
        } if label == "apps/web:hello"
    ));

    let completed: Response = serde_json::from_str(lines[2].trim_end()).expect("decode completed");
    assert!(matches!(completed, Response::RunCompleted { .. }));

    server.abort();
}
