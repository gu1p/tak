use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, RemoteSelectionSpec, RemoteSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};

struct DelayedEventsServer {
    port: u16,
    events_calls: Arc<AtomicUsize>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DelayedEventsServer {
    fn spawn(terminal_after: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed events server");
        let port = listener.local_addr().expect("listener addr").port();
        let events_calls = Arc::new(AtomicUsize::new(0));
        let events_calls_for_thread = Arc::clone(&events_calls);
        let start = Instant::now();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept request");
                let path = read_request_path(&mut stream);
                if path == "/__shutdown" {
                    write_json(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }
                if path == "/v1/node/capabilities" {
                    write_json(&mut stream, "200 OK", r#"{"compatible":true}"#);
                } else if path == "/v1/node/status" {
                    write_json(&mut stream, "200 OK", r#"{"healthy":true}"#);
                } else if path == "/v1/tasks/submit" {
                    write_json(&mut stream, "200 OK", r#"{"accepted":true}"#);
                } else if path.contains("/events") {
                    events_calls_for_thread.fetch_add(1, Ordering::SeqCst);
                    if start.elapsed() >= terminal_after {
                        write_json(
                            &mut stream,
                            "200 OK",
                            r#"{"events":[{"seq":2,"kind":"TASK_COMPLETED"}],"done":true}"#,
                        );
                    } else {
                        write_json(
                            &mut stream,
                            "200 OK",
                            r#"{"events":[{"seq":1,"kind":"TASK_LOG_CHUNK","chunk":"pending\n"}],"done":false}"#,
                        );
                    }
                } else if path.contains("/result") {
                    write_json(&mut stream, "200 OK", r#"{"success":true,"exit_code":0}"#);
                } else {
                    write_json(&mut stream, "404 Not Found", r#"{"error":"not_found"}"#);
                }
            }
        });
        Self {
            port,
            events_calls,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for DelayedEventsServer {
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

fn read_request_path(stream: &mut TcpStream) -> String {
    let mut request_line = String::new();
    BufReader::new(stream.try_clone().expect("clone stream"))
        .read_line(&mut request_line)
        .expect("read request line");
    request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string()
}

fn write_json(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
}

#[tokio::test]
async fn waits_for_delayed_terminal_events_without_busy_loop_timeout() {
    let remote = DelayedEventsServer::spawn(Duration::from_secs(2));
    let temp = tempfile::tempdir().expect("tempdir");
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "delayed_events".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "true".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-delayed-events".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks: std::collections::BTreeMap::from([(label.clone(), task)]),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed after delayed terminal event");
    assert!(
        summary.results.get(&label).expect("summary result").success,
        "task should succeed once terminal event arrives"
    );
    assert!(
        remote.events_calls.load(Ordering::SeqCst) < 80,
        "events polling should back off instead of busy-looping"
    );
}
