//! Integration tests for executor and daemon lease coordination.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tak_core::model::{
    BackoffDef, CurrentStateSpec, Hold, LimiterDef, LimiterKey, LimiterRef, NeedDef, QueueDef,
    QueueUseDef, RemoteSelectionSpec, RemoteSpec, RemoteTransportKind, ResolvedTask, RetryDef,
    Scope, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{PlacementMode, RunOptions, run_tasks};
use takd::{
    AcquireLeaseRequest, AcquireLeaseResponse, ClientInfo, NeedRequest, TaskInfo,
    new_shared_manager, run_server,
};
use tokio::time::Instant;

struct FakeRemoteProtocolServer {
    port: u16,
    handle: Option<thread::JoinHandle<()>>,
}

impl FakeRemoteProtocolServer {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote protocol server");
        let port = listener.local_addr().expect("fake remote addr").port();

        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake remote request");
                let path = read_http_request_path(&mut stream);

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }
                if path == "/v1/node/capabilities" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }
                if path == "/v1/node/status" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }
                if path == "/v1/tasks/submit" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"accepted":true}"#);
                    continue;
                }
                if path.starts_with("/v1/tasks/") && path.contains("/events") {
                    write_http_json_response(&mut stream, "200 OK", r#"{"events":[],"done":true}"#);
                    continue;
                }
                if path.starts_with("/v1/tasks/") && path.ends_with("/result") {
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        r#"{"success":true,"exit_code":0}"#,
                    );
                    continue;
                }

                write_http_json_response(&mut stream, "404 Not Found", r#"{"error":"not found"}"#);
            }
        });

        Self {
            port,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for FakeRemoteProtocolServer {
    fn drop(&mut self) {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
            let _ = stream.write_all(
                b"GET /__shutdown HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            );
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_http_request_path(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .expect("read request line");
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).expect("read header");
        if header == "\r\n" || header == "\n" || header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().unwrap_or(0);
        }
    }

    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).expect("read request body");
    path
}

fn write_http_json_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
}

/// Builds a single-step task fixture that requires the given needs.
fn make_task(
    label: TaskLabel,
    needs: Vec<NeedDef>,
    log_file: &std::path::Path,
    execution: TaskExecutionSpec,
) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo run >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs,
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef {
            attempts: 1,
            on_exit: Vec::new(),
            backoff: BackoffDef::Fixed { seconds: 0.0 },
        },
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution,
        tags: Vec::new(),
    }
}

/// Verifies the executor waits on pending leases and releases granted leases on completion.
#[tokio::test]
async fn run_waits_for_lease_then_releases_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let log_file = temp.path().join("run.log");

    let manager = new_shared_manager();
    {
        let mut guard = manager.lock().expect("manager lock");
        guard.set_capacity("cpu", Scope::Machine, None, 1.0);
    }

    let held_lease_id = {
        let request = AcquireLeaseRequest {
            request_id: "hold-request".to_string(),
            client: ClientInfo {
                user: "alice".to_string(),
                pid: 1,
                session_id: "session-1".to_string(),
            },
            task: TaskInfo {
                label: "//:hold".to_string(),
                attempt: 1,
            },
            needs: vec![NeedRequest {
                name: "cpu".to_string(),
                scope: Scope::Machine,
                scope_key: None,
                slots: 1.0,
            }],
            ttl_ms: 30_000,
        };

        let mut guard = manager.lock().expect("manager lock");
        match guard.acquire(request) {
            AcquireLeaseResponse::LeaseGranted { lease } => lease.lease_id,
            AcquireLeaseResponse::LeasePending { .. } => panic!("expected initial lease grant"),
        }
    };

    let server_manager = Arc::clone(&manager);
    let server_socket = socket_path.clone();
    let server = tokio::spawn(async move {
        run_server(&server_socket, server_manager)
            .await
            .expect("server should run")
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(socket_path.exists(), "socket should exist");

    let release_manager = Arc::clone(&manager);
    let lease_to_release = held_lease_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let mut guard = release_manager.lock().expect("manager lock");
        guard
            .release(&lease_to_release)
            .expect("holding lease should be released");
    });

    let label = TaskLabel {
        package: "//".to_string(),
        name: "run".to_string(),
    };

    let task = make_task(
        label.clone(),
        vec![NeedDef {
            limiter: LimiterRef {
                name: "cpu".to_string(),
                scope: Scope::Machine,
                scope_key: None,
            },
            slots: 1.0,
            hold: Hold::During,
        }],
        &log_file,
        TaskExecutionSpec::default(),
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-lease-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let started = Instant::now();
    let summary = run_tasks(
        &spec,
        &[label],
        &RunOptions {
            jobs: 1,
            keep_going: false,
            lease_socket: Some(socket_path),
            lease_ttl_ms: 5_000,
            lease_poll_interval_ms: 50,
            session_id: Some("session-run".to_string()),
            user: Some("alice".to_string()),
        },
    )
    .await
    .expect("run should succeed after lease wait");

    assert!(started.elapsed() >= Duration::from_millis(200));

    let result = summary.results.values().next().expect("result exists");
    assert!(result.success);

    let log = fs::read_to_string(log_file).expect("log should exist");
    assert_eq!(log.lines().collect::<Vec<_>>(), vec!["run"]);

    let mut guard = manager.lock().expect("manager lock");
    let status = guard.status();
    assert_eq!(status.active_leases, 0);

    server.abort();
}

/// Verifies remote-enabled task runs preserve lease behavior and still release granted leases.
#[tokio::test]
async fn run_remote_task_with_needs_releases_lease_and_preserves_remote_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn();

    let manager = new_shared_manager();
    {
        let mut guard = manager.lock().expect("manager lock");
        guard.set_capacity("cpu", Scope::Machine, None, 1.0);
    }

    let server_manager = Arc::clone(&manager);
    let server_socket = socket_path.clone();
    let server = tokio::spawn(async move {
        run_server(&server_socket, server_manager)
            .await
            .expect("server should run")
    });

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(socket_path.exists(), "socket should exist");

    let label = TaskLabel {
        package: "//".to_string(),
        name: "remote_run".to_string(),
    };

    let task = make_task(
        label.clone(),
        vec![NeedDef {
            limiter: LimiterRef {
                name: "cpu".to_string(),
                scope: Scope::Machine,
                scope_key: None,
            },
            slots: 1.0,
            hold: Hold::During,
        }],
        &log_file,
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-lease-node".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-lease-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&label),
        &RunOptions {
            jobs: 1,
            keep_going: false,
            lease_socket: Some(socket_path),
            lease_ttl_ms: 5_000,
            lease_poll_interval_ms: 50,
            session_id: Some("session-run".to_string()),
            user: Some("alice".to_string()),
        },
    )
    .await
    .expect("remote-enabled run should succeed");

    let result = summary.results.get(&label).expect("result exists");
    assert!(result.success);
    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(
        result.remote_node_id.as_deref(),
        Some("remote-lease-node"),
        "remote placement should remain visible in summary metadata"
    );

    assert!(
        !log_file.exists(),
        "strict remote lease flow should not execute task command locally"
    );

    let mut guard = manager.lock().expect("manager lock");
    let status = guard.status();
    assert_eq!(status.active_leases, 0);

    server.abort();
}
