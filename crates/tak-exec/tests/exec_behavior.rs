//! Behavioral tests for local executor ordering and retry contracts.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use tak_core::model::{
    BackoffDef, CurrentStateSpec, IgnoreSourceSpec, LimiterKey, NeedDef, QueueDef, QueueUseDef,
    RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec, RemoteTransportKind, ResolvedTask,
    RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec, build_current_state_manifest,
    normalize_path_ref,
};
use tak_exec::{PlacementMode, RunOptions, run_tasks};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Copy, Debug)]
struct FakeRemoteProtocolConfig {
    preflight_compatible: bool,
    result_success: bool,
    result_exit_code: i32,
}

struct FakeRemoteProtocolServer {
    port: u16,
    call_order: Arc<Mutex<Vec<String>>>,
    request_paths: Arc<Mutex<Vec<String>>>,
    submit_payloads: Arc<Mutex<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct FakeRemoteStreamingServer {
    port: u16,
    call_order: Arc<Mutex<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct FakeRemoteSubmitServer {
    port: u16,
    call_order: Arc<Mutex<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct FakeRemoteAuthHeaderServer {
    port: u16,
    call_order: Arc<Mutex<Vec<String>>>,
    request_headers: Arc<Mutex<Vec<HashMap<String, String>>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl FakeRemoteStreamingServer {
    fn spawn(events_responses: Vec<String>, result_response: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake streaming remote");
        let port = listener.local_addr().expect("listener addr").port();
        let call_order = Arc::new(Mutex::new(Vec::new()));
        let call_order_for_thread = Arc::clone(&call_order);

        let handle = thread::spawn(move || {
            let mut events_request_index = 0_usize;
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake remote request");
                let (path, _) = read_http_request(&mut stream);

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }

                if path.starts_with("/v1/preflight") {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("preflight".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/capabilities" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("capabilities".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/status" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("status".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }

                if path.starts_with("/v1/submit") || path == "/v1/tasks/submit" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("submit".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"accepted":true}"#);
                    continue;
                }

                if path.starts_with("/v1/events")
                    || (path.starts_with("/v1/tasks/") && path.contains("/events"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("events".to_string());
                    let response = events_responses
                        .get(events_request_index)
                        .cloned()
                        .unwrap_or_else(|| r#"{"events":[],"done":true}"#.to_string());
                    events_request_index += 1;
                    write_http_json_response(&mut stream, "200 OK", &response);
                    continue;
                }

                if path.starts_with("/v1/result")
                    || (path.starts_with("/v1/tasks/") && path.ends_with("/result"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("result".to_string());
                    write_http_json_response(&mut stream, "200 OK", &result_response);
                    continue;
                }

                write_http_json_response(&mut stream, "404 Not Found", r#"{"error":"not found"}"#);
            }
        });

        Self {
            port,
            call_order,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.call_order.lock().expect("lock call order").clone()
    }
}

impl Drop for FakeRemoteStreamingServer {
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

impl FakeRemoteSubmitServer {
    fn auth_rejecting() -> Self {
        Self::spawn(
            "401 Unauthorized",
            r#"{"accepted":false,"reason":"auth_failed"}"#,
        )
    }

    fn spawn(submit_status: &'static str, submit_body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake auth remote");
        let port = listener.local_addr().expect("listener addr").port();
        let call_order = Arc::new(Mutex::new(Vec::new()));
        let call_order_for_thread = Arc::clone(&call_order);
        let submit_status = submit_status.to_string();
        let submit_body = submit_body.to_string();

        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake remote request");
                let (path, _) = read_http_request(&mut stream);

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }

                if path.starts_with("/v1/preflight") {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("preflight".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/capabilities" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("capabilities".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/status" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("status".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }

                if path.starts_with("/v1/submit") || path == "/v1/tasks/submit" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("submit".to_string());
                    write_http_json_response(&mut stream, &submit_status, &submit_body);
                    continue;
                }

                if path.starts_with("/v1/events")
                    || (path.starts_with("/v1/tasks/") && path.contains("/events"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("events".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        r#"{"events":[{"seq":1,"kind":"TASK_LOG_CHUNK"}],"done":true}"#,
                    );
                    continue;
                }

                if path.starts_with("/v1/result")
                    || (path.starts_with("/v1/tasks/") && path.ends_with("/result"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("result".to_string());
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
            call_order,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.call_order.lock().expect("lock call order").clone()
    }
}

impl Drop for FakeRemoteSubmitServer {
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

impl FakeRemoteAuthHeaderServer {
    fn spawn(expected_service_token: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake auth-header remote");
        let port = listener.local_addr().expect("listener addr").port();
        let call_order = Arc::new(Mutex::new(Vec::new()));
        let request_headers = Arc::new(Mutex::new(Vec::new()));
        let call_order_for_thread = Arc::clone(&call_order);
        let request_headers_for_thread = Arc::clone(&request_headers);
        let expected_service_token = expected_service_token.to_string();

        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake remote request");
                let (path, headers, _) = read_http_request_with_headers(&mut stream);

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }

                // The executor performs a plain TCP reachability probe before protocol requests.
                if path == "/" && headers.is_empty() {
                    continue;
                }

                request_headers_for_thread
                    .lock()
                    .expect("lock request headers")
                    .push(headers.clone());

                let protocol = headers.get("x-tak-protocol-version").map(String::as_str);
                let service_token = headers.get("x-tak-service-token").map(String::as_str);
                let auth_ok = protocol == Some("v1")
                    && service_token == Some(expected_service_token.as_str());
                if !auth_ok {
                    write_http_json_response(
                        &mut stream,
                        "401 Unauthorized",
                        r#"{"accepted":false,"reason":"auth_failed"}"#,
                    );
                    continue;
                }

                if path == "/v1/node/capabilities" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("capabilities".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/status" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("status".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }

                if path == "/v1/tasks/submit" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("submit".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"accepted":true}"#);
                    continue;
                }

                if path.starts_with("/v1/tasks/") && path.contains("/events") {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("events".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        r#"{"events":[{"seq":1,"kind":"TASK_LOG_CHUNK","chunk":"ok\n"}],"done":true}"#,
                    );
                    continue;
                }

                if path.starts_with("/v1/tasks/") && path.ends_with("/result") {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("result".to_string());
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
            call_order,
            request_headers,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.call_order.lock().expect("lock call order").clone()
    }

    fn request_headers(&self) -> Vec<HashMap<String, String>> {
        self.request_headers
            .lock()
            .expect("lock request headers")
            .clone()
    }
}

impl Drop for FakeRemoteAuthHeaderServer {
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

impl FakeRemoteProtocolServer {
    fn spawn(config: FakeRemoteProtocolConfig) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote protocol server");
        let port = listener.local_addr().expect("listener addr").port();
        let call_order = Arc::new(Mutex::new(Vec::new()));
        let request_paths = Arc::new(Mutex::new(Vec::new()));
        let submit_payloads = Arc::new(Mutex::new(Vec::new()));
        let call_order_for_thread = Arc::clone(&call_order);
        let request_paths_for_thread = Arc::clone(&request_paths);
        let submit_payloads_for_thread = Arc::clone(&submit_payloads);

        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake remote request");
                let (path, body) = read_http_request(&mut stream);
                request_paths_for_thread
                    .lock()
                    .expect("lock request paths")
                    .push(path.clone());

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }

                if path.starts_with("/v1/preflight") {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("preflight".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        &format!(r#"{{"compatible":{}}}"#, config.preflight_compatible),
                    );
                    continue;
                }

                if path == "/v1/node/capabilities" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("capabilities".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        &format!(r#"{{"compatible":{}}}"#, config.preflight_compatible),
                    );
                    continue;
                }

                if path == "/v1/node/status" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("status".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }

                if path.starts_with("/v1/submit") || path == "/v1/tasks/submit" {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("submit".to_string());
                    submit_payloads_for_thread
                        .lock()
                        .expect("lock submit payloads")
                        .push(body);
                    write_http_json_response(&mut stream, "200 OK", r#"{"accepted":true}"#);
                    continue;
                }

                if path.starts_with("/v1/events")
                    || (path.starts_with("/v1/tasks/") && path.contains("/events"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("events".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        r#"{"events":[{"seq":1,"kind":"TASK_LOG_CHUNK"}],"done":true}"#,
                    );
                    continue;
                }

                if path.starts_with("/v1/result")
                    || (path.starts_with("/v1/tasks/") && path.ends_with("/result"))
                {
                    call_order_for_thread
                        .lock()
                        .expect("lock call order")
                        .push("result".to_string());
                    write_http_json_response(
                        &mut stream,
                        "200 OK",
                        &format!(
                            r#"{{"success":{},"exit_code":{}}}"#,
                            config.result_success, config.result_exit_code
                        ),
                    );
                    continue;
                }

                write_http_json_response(&mut stream, "404 Not Found", r#"{"error":"not found"}"#);
            }
        });

        Self {
            port,
            call_order,
            request_paths,
            submit_payloads,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.call_order.lock().expect("lock call order").clone()
    }

    fn submit_payloads(&self) -> Vec<serde_json::Value> {
        self.submit_payloads
            .lock()
            .expect("lock submit payloads")
            .iter()
            .map(|payload| serde_json::from_str(payload).expect("parse submit payload"))
            .collect()
    }

    fn request_paths(&self) -> Vec<String> {
        self.request_paths
            .lock()
            .expect("lock request paths")
            .clone()
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

fn read_http_request_with_headers(
    stream: &mut TcpStream,
) -> (String, HashMap<String, String>, String) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .expect("read HTTP request line");

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .expect("read HTTP header line");
        if header == "\r\n" || header == "\n" || header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let key = name.trim().to_ascii_lowercase();
            let normalized_value = value.trim().to_string();
            if key == "content-length" {
                content_length = normalized_value.parse::<usize>().unwrap_or(0);
            }
            headers.insert(key, normalized_value);
        }
    }

    let mut body_bytes = vec![0_u8; content_length];
    reader
        .read_exact(&mut body_bytes)
        .expect("read HTTP request body");
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    (path, headers, body)
}

fn read_http_request(stream: &mut TcpStream) -> (String, String) {
    let (path, _, body) = read_http_request_with_headers(stream);
    (path, body)
}

fn write_http_json_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write HTTP response");
}

fn write_fake_engine_binary(dir: &Path, name: &str, exit_code: i32) {
    fs::create_dir_all(dir).expect("create fake engine dir");
    let binary_path = dir.join(name);
    let script = format!("#!/bin/sh\nexit {exit_code}\n");
    fs::write(&binary_path, script).expect("write fake engine binary");
    let mut permissions = fs::metadata(&binary_path)
        .expect("read fake engine metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&binary_path, permissions).expect("set fake engine mode");
}

fn write_fake_engine_binary_with_probe_log(
    dir: &Path,
    name: &str,
    exit_code: i32,
    probe_log: &Path,
) {
    fs::create_dir_all(dir).expect("create fake engine dir");
    let binary_path = dir.join(name);
    let script = format!(
        "#!/bin/sh\necho {name} >> '{}'\nexit {exit_code}\n",
        probe_log.display()
    );
    fs::write(&binary_path, script).expect("write fake engine binary");
    let mut permissions = fs::metadata(&binary_path)
        .expect("read fake engine metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&binary_path, permissions).expect("set fake engine mode");
}

fn prepend_path(prefix: &Path) -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    format!("{}:{current_path}", prefix.display())
}

async fn env_var_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| AsyncMutex::new(())).lock().await
}

struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: String) -> Self {
        let previous = std::env::var(key).ok();
        // SAFETY: tests in this module only mutate env vars in tightly scoped guards and restore
        // previous values before exit.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => {
                // SAFETY: restoration mirrors `set` above and happens during guard drop.
                unsafe {
                    std::env::set_var(self.key, value);
                }
            }
            None => {
                // SAFETY: restoration mirrors `set` above and happens during guard drop.
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }
}

/// Constructs a minimal resolved task used by executor tests.
fn task(
    label: TaskLabel,
    deps: Vec<TaskLabel>,
    steps: Vec<StepDef>,
    retry: RetryDef,
) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps,
        steps,
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry,
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::default(),
        tags: Vec::new(),
    }
}

/// Verifies dependency tasks execute before dependent targets.
#[tokio::test]
async fn executes_dependencies_before_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    let build_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "build".to_string(),
    };
    let test_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };

    let build = task(
        build_label.clone(),
        Vec::new(),
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo build >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        RetryDef::default(),
    );

    let test = task(
        test_label.clone(),
        vec![build_label.clone()],
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo test >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        RetryDef::default(),
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(build_label.clone(), build);
    tasks.insert(test_label.clone(), test);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    run_tasks(&spec, &[test_label], &RunOptions::default())
        .await
        .expect("run should succeed");

    let log = fs::read_to_string(log_file).expect("read log");
    assert_eq!(log.lines().collect::<Vec<_>>(), vec!["build", "test"]);
}

/// Verifies retry behavior when a task exits with a configured retriable exit code.
#[tokio::test]
async fn retries_failed_task_when_exit_code_matches_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker = temp.path().join("first_attempt_seen");

    let label = TaskLabel {
        package: "//".to_string(),
        name: "flaky".to_string(),
    };

    let retry = RetryDef {
        attempts: 2,
        on_exit: vec![42],
        backoff: BackoffDef::Fixed { seconds: 0.0 },
    };

    let flaky = task(
        label.clone(),
        Vec::new(),
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "if [ -f {0} ]; then exit 0; else touch {0}; exit 42; fi",
                    marker.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        retry,
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), flaky);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, &[label], &RunOptions::default())
        .await
        .expect("run should succeed after retry");

    let result = summary.results.values().next().expect("result exists");
    assert_eq!(result.attempts, 2);
    assert!(result.success);
}

/// Verifies remote dispatch sends attempt identity and selected node, and lifecycle order is stable.
#[tokio::test]
async fn remote_only_single_dispatches_identity_and_selected_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_dispatch".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo remote_dispatch >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");

    let result = summary.results.get(&label).expect("summary contains task");
    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(result.remote_node_id.as_deref(), Some("remote-primary"));

    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "remote lifecycle should remain capabilities->status->submit->events->result"
    );

    let submit_payloads = remote.submit_payloads();
    assert_eq!(submit_payloads.len(), 1, "expected one submit payload");
    let submit = &submit_payloads[0];
    assert!(
        submit
            .get("task_run_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty()),
        "submit should include non-empty task_run_id"
    );
    assert_eq!(
        submit.get("attempt").and_then(serde_json::Value::as_u64),
        Some(1),
        "submit should include first-attempt index"
    );
    assert_eq!(
        submit
            .get("selected_node_id")
            .and_then(serde_json::Value::as_str),
        Some("remote-primary"),
        "submit should include selected strict remote node id"
    );
    assert_eq!(
        submit.get("task_label").and_then(serde_json::Value::as_str),
        Some("apps/web:remote_dispatch"),
        "submit should preserve existing task label field"
    );
}

/// Verifies remote protocol client uses canonical V1 endpoint paths.
#[tokio::test]
async fn remote_only_single_uses_canonical_v1_endpoint_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_canonical_paths".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "echo ok".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");

    let paths = remote.request_paths();
    assert!(
        paths.iter().any(|path| path == "/v1/node/capabilities"),
        "expected canonical capabilities path, got: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path == "/v1/node/status"),
        "expected canonical node status path, got: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path == "/v1/tasks/submit"),
        "expected canonical submit path, got: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.starts_with("/v1/tasks/") && path.contains("/events")),
        "expected canonical events path, got: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.starts_with("/v1/tasks/") && path.ends_with("/result")),
        "expected canonical result path, got: {paths:?}"
    );
}

/// Verifies remote result envelope drives terminal failure status surfaced by executor interfaces.
#[tokio::test]
async fn remote_only_single_maps_remote_result_failure_to_terminal_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: false,
        result_exit_code: 42,
    });

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_result_failure".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label, task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(
        &spec,
        &[TaskLabel {
            package: "//apps/web".to_string(),
            name: "remote_result_failure".to_string(),
        }],
        &RunOptions::default(),
    )
    .await
    .expect_err("run should fail when remote result envelope reports failure");
    let message = format!("{err:#}");
    assert!(
        message.contains("task apps/web:remote_result_failure failed"),
        "executor should map remote failure result into existing terminal error interface: {message}"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "executor should still complete full remote protocol lifecycle before surfacing failure"
    );
}

/// Verifies strict remote unavailability is surfaced as infra error in executor integration.
#[tokio::test]
async fn remote_only_single_unavailable_endpoint_returns_infra_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_unavailable".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-down".to_string(),
            endpoint: Some("http://127.0.0.1:9".to_string()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label, task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(
        &spec,
        &[TaskLabel {
            package: "//apps/web".to_string(),
            name: "remote_unavailable".to_string(),
        }],
        &RunOptions::default(),
    )
    .await
    .expect_err("run should fail when strict remote endpoint is unavailable");
    let message = format!("{err:#}");
    assert!(
        message.contains("infra error: remote node remote-down unavailable"),
        "executor should preserve explicit strict-remote infra error context: {message}"
    );
}

/// Verifies strict remote auth rejection surfaces infra auth error and does not fallback.
#[tokio::test]
async fn remote_only_single_auth_rejection_returns_infra_auth_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let rejecting_remote = FakeRemoteSubmitServer::auth_rejecting();
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_auth_strict".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-auth".to_string(),
            endpoint: Some(rejecting_remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("strict remote auth rejection should fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("auth failed"),
        "strict auth rejection should surface explicit auth failure reason: {message}"
    );
    assert_eq!(
        rejecting_remote.call_order(),
        vec!["capabilities", "status", "submit"],
        "strict auth rejection should stop before events/result"
    );
}

/// Verifies direct transport sends canonical protocol + service auth headers for remote requests.
#[tokio::test]
async fn remote_only_single_sends_protocol_and_service_auth_headers() {
    let _env_lock = env_var_lock().await;
    let _token_guard = ScopedEnvVar::set("TAK_TEST_REMOTE_SERVICE_TOKEN", "token-abc".to_string());
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteAuthHeaderServer::spawn("token-abc");
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_auth_headers".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-auth-headers".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: Some("TAK_TEST_REMOTE_SERVICE_TOKEN".to_string()),
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed with valid service auth token");
    let result = summary.results.get(&label).expect("summary contains task");
    assert!(result.success);
    assert_eq!(result.placement_mode, PlacementMode::Remote);

    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "remote lifecycle should complete when protocol/auth headers are present"
    );

    let request_headers = remote.request_headers();
    assert!(
        request_headers.iter().all(|headers| {
            headers
                .get("x-tak-protocol-version")
                .is_some_and(|value| value == "v1")
        }),
        "every remote request should include canonical protocol marker header"
    );
    assert!(
        request_headers.iter().all(|headers| {
            headers
                .get("x-tak-service-token")
                .is_some_and(|value| value == "token-abc")
        }),
        "every remote request should include node-scoped service auth header"
    );
}

/// Verifies strict remote auth failures during capabilities preflight surface explicit infra errors.
#[tokio::test]
async fn remote_only_single_auth_failure_during_capabilities_returns_infra_error() {
    let _env_lock = env_var_lock().await;
    let _token_guard =
        ScopedEnvVar::set("TAK_TEST_REMOTE_SERVICE_TOKEN", "wrong-token".to_string());
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteAuthHeaderServer::spawn("expected-token");
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_auth_capabilities".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-auth-capabilities".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: Some("TAK_TEST_REMOTE_SERVICE_TOKEN".to_string()),
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("auth failure during capabilities preflight should fail run");
    let message = format!("{err:#}");
    assert!(
        message.contains("auth failed"),
        "auth failure should be surfaced explicitly instead of downgrading handshake: {message}"
    );
    assert_eq!(
        remote.call_order(),
        Vec::<String>::new(),
        "server should reject before any phase marker is recorded"
    );
}

async fn assert_service_token_redaction_on_invalid_header_value(
    transport_kind: RemoteTransportKind,
    token_env: &'static str,
) {
    let _env_lock = env_var_lock().await;
    let secret = "super-secret-token";
    let _token_guard = ScopedEnvVar::set(token_env, format!("{secret}\ninvalid"));
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });
    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "service_token_redaction".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-redaction".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind,
            service_auth_env: Some(token_env.to_string()),
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("invalid service auth token value should fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("contains invalid characters"),
        "error should classify invalid header-safe service token values: {message}"
    );
    assert!(
        message.contains(token_env),
        "error should identify token env key without leaking value: {message}"
    );
    assert!(
        !message.contains(secret),
        "error should redact service token value for all transports: {message}"
    );
}

/// Verifies direct transport errors redact raw service token values from diagnostics.
#[tokio::test]
async fn direct_transport_service_token_errors_are_redacted() {
    assert_service_token_redaction_on_invalid_header_value(
        RemoteTransportKind::DirectHttps,
        "TAK_TEST_DIRECT_SERVICE_TOKEN",
    )
    .await;
}

/// Verifies Tor transport errors redact raw service token values from diagnostics.
#[tokio::test]
async fn tor_transport_service_token_errors_are_redacted() {
    assert_service_token_redaction_on_invalid_header_value(
        RemoteTransportKind::Tor,
        "TAK_TEST_TOR_SERVICE_TOKEN",
    )
    .await;
}

/// Verifies ordered remote fallback advances when first node rejects submit with auth failure.
#[tokio::test]
async fn remote_only_list_falls_back_when_first_node_auth_rejects_submit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let auth_rejecting_remote = FakeRemoteSubmitServer::auth_rejecting();
    let fallback_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_auth_fallback".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(vec![
            RemoteSpec {
                id: "remote-auth-reject".to_string(),
                endpoint: Some(auth_rejecting_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: None,
            },
            RemoteSpec {
                id: "remote-fallback".to_string(),
                endpoint: Some(fallback_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: None,
            },
        ])),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("fallback remote should run after first auth rejection");
    let result = summary
        .results
        .get(&label)
        .expect("summary should contain task result");
    assert!(result.success, "fallback run should succeed");
    assert_eq!(
        result.remote_node_id.as_deref(),
        Some("remote-fallback"),
        "ordered fallback should advance to next configured node on auth rejection"
    );
    assert_eq!(
        auth_rejecting_remote.call_order(),
        vec!["capabilities", "status", "submit"],
        "first node should fail during submit auth check"
    );
    assert_eq!(
        fallback_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "second node should complete normal protocol lifecycle"
    );
}

/// Verifies ordered fallback surfaces auth-focused infra error when all nodes reject auth.
#[tokio::test]
async fn remote_only_list_all_auth_rejections_return_auth_infra_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let first_remote = FakeRemoteSubmitServer::auth_rejecting();
    let second_remote = FakeRemoteSubmitServer::auth_rejecting();

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_auth_all_reject".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(vec![
            RemoteSpec {
                id: "remote-auth-a".to_string(),
                endpoint: Some(first_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: None,
            },
            RemoteSpec {
                id: "remote-auth-b".to_string(),
                endpoint: Some(second_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: None,
            },
        ])),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("all auth-rejecting remotes should fail");
    let message = format!("{err:#}");
    assert!(
        message.contains("auth failed"),
        "error should distinguish auth rejection from connectivity failures: {message}"
    );
    assert_eq!(
        first_remote.call_order(),
        vec!["capabilities", "status", "submit"],
        "first node should be attempted before fallback"
    );
    assert_eq!(
        second_remote.call_order(),
        vec!["capabilities", "status", "submit"],
        "second node should also be attempted in ordered fallback mode"
    );
}

/// Verifies remote execution stages only manifest-selected files in task workspace.
#[tokio::test]
async fn remote_execution_stages_only_current_state_manifest_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let listed_files = temp.path().join("listed-files.txt");
    let project_dir = temp.path().join("apps/web/project");
    let ignored_dir = project_dir.join("ignored");
    fs::create_dir_all(&ignored_dir).expect("mkdir ignored");
    fs::create_dir_all(temp.path().join("apps/web/outside")).expect("mkdir outside");
    fs::write(project_dir.join("keep.txt"), "keep\n").expect("write keep");
    fs::write(ignored_dir.join("drop.txt"), "drop\n").expect("write drop");
    fs::write(ignored_dir.join("reinclude.txt"), "reinclude\n").expect("write reinclude");
    fs::write(
        temp.path().join("apps/web/outside/should_not_transfer.txt"),
        "outside\n",
    )
    .expect("write outside");

    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_context".to_string(),
    };

    let context = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root path")],
        ignored: vec![IgnoreSourceSpec::Path(
            normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored path"),
        )],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include path"),
            normalize_path_ref("workspace", "apps/web/outside/should_not_transfer.txt")
                .expect("outside include path"),
        ],
    };

    let expected_manifest = build_current_state_manifest(
        vec![
            normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep ref"),
            normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop ref"),
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("reinclude ref"),
            normalize_path_ref("workspace", "apps/web/outside/should_not_transfer.txt")
                .expect("outside ref"),
        ],
        &context,
    );

    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "find . -type f | LC_ALL=C sort > {}",
                    listed_files.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(format!("http://127.0.0.1:{remote_port}")),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        context,
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");

    let listed = fs::read_to_string(&listed_files).expect("listed files output exists");
    let staged_files = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_start_matches("./").to_string())
        .collect::<BTreeSet<_>>();
    let expected_files = expected_manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        staged_files, expected_files,
        "staged payload files must exactly match computed ContextManifest entries"
    );
    assert!(
        !staged_files.contains("apps/web/outside/should_not_transfer.txt"),
        "files outside selected roots should never be transferred"
    );

    let result = summary
        .results
        .get(&label)
        .expect("summary contains result");
    assert_eq!(
        result.context_manifest_hash.as_deref(),
        Some(expected_manifest.hash.as_str()),
        "context hash associated with staged payload should match manifest hash"
    );
}

/// Verifies containerized remote execution stages only manifest files and syncs outputs/logs back.
#[tokio::test]
async fn remote_container_runtime_stages_manifest_and_syncs_outputs_and_logs() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let listed_files = temp.path().join("staged-files.txt");
    let synced_output = temp.path().join("dist/result.txt");
    let project_dir = temp.path().join("apps/web/project");
    let ignored_dir = project_dir.join("ignored");
    fs::create_dir_all(&ignored_dir).expect("mkdir ignored");
    fs::create_dir_all(temp.path().join("apps/web/outside")).expect("mkdir outside");
    fs::write(project_dir.join("keep.txt"), "keep\n").expect("write keep");
    fs::write(ignored_dir.join("drop.txt"), "drop\n").expect("write drop");
    fs::write(ignored_dir.join("reinclude.txt"), "reinclude\n").expect("write reinclude");
    fs::write(
        temp.path().join("apps/web/outside/private.txt"),
        "private\n",
    )
    .expect("write outside");

    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 0);
    write_fake_engine_binary(&fake_bin_dir, "podman", 0);
    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "other".to_string());

    let remote = FakeRemoteStreamingServer::spawn(
        vec![
            r#"{"events":[{"seq":7,"kind":"TASK_LOG_CHUNK","chunk":"container-log\n"}],"done":true}"#
                .to_string(),
        ],
        r#"{
          "success": true,
          "exit_code": 0,
          "sync_mode": "OUTPUTS_AND_LOGS",
          "outputs": [
            {"path":"dist/result.txt","digest":"sha256:0d510613db874c8f4a366d040c392ae9941685877df8343ccff5d93d239ea547","size":15}
          ]
        }"#
        .to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_manifest_and_sync".to_string(),
    };

    let context = CurrentStateSpec {
        roots: vec![normalize_path_ref("workspace", "apps/web/project").expect("root path")],
        ignored: vec![IgnoreSourceSpec::Path(
            normalize_path_ref("workspace", "apps/web/project/ignored").expect("ignored path"),
        )],
        include: vec![
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("include path"),
            normalize_path_ref("workspace", "apps/web/outside/private.txt")
                .expect("outside include path"),
        ],
    };

    let expected_manifest = build_current_state_manifest(
        vec![
            normalize_path_ref("workspace", "apps/web/project/keep.txt").expect("keep ref"),
            normalize_path_ref("workspace", "apps/web/project/ignored/drop.txt").expect("drop ref"),
            normalize_path_ref("workspace", "apps/web/project/ignored/reinclude.txt")
                .expect("reinclude ref"),
            normalize_path_ref("workspace", "apps/web/outside/private.txt").expect("outside ref"),
        ],
        &context,
    );

    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "find . -type f | LC_ALL=C sort > '{listed}'; \
                     [ -f apps/web/project/keep.txt ]; \
                     [ ! -f apps/web/project/ignored/drop.txt ]; \
                     [ -f apps/web/project/ignored/reinclude.txt ]; \
                     [ ! -f apps/web/outside/private.txt ]; \
                     mkdir -p dist; \
                     cat apps/web/project/keep.txt apps/web/project/ignored/reinclude.txt > dist/result.txt",
                    listed = listed_files.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized {
                image: "tak/test:v1".to_string(),
            }),
        })),
        context,
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");
    let result = summary
        .results
        .get(&label)
        .expect("summary contains result");

    let listed = fs::read_to_string(&listed_files).expect("listed files output exists");
    let staged_files = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_start_matches("./").to_string())
        .collect::<BTreeSet<_>>();
    let expected_files = expected_manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        staged_files, expected_files,
        "containerized staged workspace must match the computed ContextManifest"
    );
    assert!(
        !staged_files.contains("apps/web/outside/private.txt"),
        "files outside selected roots must remain unavailable inside container workspace"
    );

    assert_eq!(result.remote_runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.remote_runtime_engine.as_deref(), Some("docker"));
    assert_eq!(
        result.context_manifest_hash.as_deref(),
        Some(expected_manifest.hash.as_str())
    );
    assert_eq!(
        result
            .remote_logs
            .iter()
            .map(|chunk| (chunk.seq, chunk.chunk.as_str()))
            .collect::<Vec<_>>(),
        vec![(7, "container-log\n")],
        "containerized remote logs should be persisted with standard remote stream semantics"
    );
    assert_eq!(
        result
            .synced_outputs
            .iter()
            .map(|output| (
                output.path.as_str(),
                output.digest.as_str(),
                output.size_bytes
            ))
            .collect::<Vec<_>>(),
        vec![(
            "dist/result.txt",
            "sha256:0d510613db874c8f4a366d040c392ae9941685877df8343ccff5d93d239ea547",
            15
        )],
        "containerized path should preserve OUTPUTS_AND_LOGS metadata contract"
    );

    let output_contents =
        fs::read_to_string(&synced_output).expect("synced output should be available locally");
    assert_eq!(
        output_contents, "keep\nreinclude\n",
        "synced output should preserve container workspace artifact content"
    );
}

/// Verifies integration-level container engine selection prefers Docker without probing Podman.
#[tokio::test]
async fn remote_container_runtime_prefers_docker_without_probings_podman() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "docker", 0, &probe_log);
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "podman", 0, &probe_log);

    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "other".to_string());

    let remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_engine_docker".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "echo runtime=$TAK_REMOTE_RUNTIME engine=$TAK_REMOTE_ENGINE image=$TAK_REMOTE_CONTAINER_IMAGE >> '{}'",
                    marker_file.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container-docker".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized {
                image: "tak/test:v1".to_string(),
            }),
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");
    let result = summary
        .results
        .get(&label)
        .expect("summary contains result");

    let marker = fs::read_to_string(&marker_file).expect("runtime marker should be written");
    assert!(
        marker.contains("runtime=containerized"),
        "runtime marker should indicate containerized execution: {marker}"
    );
    assert!(
        marker.contains("engine=docker"),
        "runtime marker should include docker engine: {marker}"
    );
    assert!(
        marker.contains("image=tak/test:v1"),
        "runtime marker should include selected image: {marker}"
    );
    assert_eq!(
        result.remote_runtime_kind.as_deref(),
        Some("containerized"),
        "result should persist runtime kind placement metadata"
    );
    assert_eq!(
        result.remote_runtime_engine.as_deref(),
        Some("docker"),
        "result should persist selected docker engine metadata"
    );

    let probes = fs::read_to_string(&probe_log).expect("probe log should exist");
    assert_eq!(
        probes.lines().collect::<Vec<_>>(),
        vec!["docker"],
        "docker probe must short-circuit and avoid podman probing when available"
    );
}

/// Verifies macOS runtime selection falls back to Podman after Docker probe failure.
#[tokio::test]
async fn remote_container_runtime_falls_back_to_podman_on_macos() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "docker", 1, &probe_log);
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "podman", 0, &probe_log);

    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "macos".to_string());

    let remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_engine_podman".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "echo runtime=$TAK_REMOTE_RUNTIME engine=$TAK_REMOTE_ENGINE >> '{}'",
                    marker_file.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container-podman".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized {
                image: "tak/test:v1".to_string(),
            }),
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed with podman fallback");
    let result = summary
        .results
        .get(&label)
        .expect("summary contains result");

    let probes = fs::read_to_string(&probe_log).expect("probe log should exist");
    assert_eq!(
        probes.lines().collect::<Vec<_>>(),
        vec!["docker", "podman"],
        "macos selection should probe docker first then podman"
    );

    let marker = fs::read_to_string(&marker_file).expect("runtime marker should be written");
    assert!(
        marker.contains("runtime=containerized"),
        "runtime marker should indicate containerized execution: {marker}"
    );
    assert!(
        marker.contains("engine=podman"),
        "runtime marker should include podman engine after fallback: {marker}"
    );
    assert_eq!(
        result.remote_runtime_kind.as_deref(),
        Some("containerized"),
        "result should persist runtime kind placement metadata"
    );
    assert_eq!(
        result.remote_runtime_engine.as_deref(),
        Some("podman"),
        "result should persist selected podman engine metadata"
    );
}

/// Verifies unavailable engines surface explicit infra diagnostics with attempted probe order.
#[tokio::test]
async fn remote_container_runtime_unavailable_lists_attempted_engine_probes() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "docker", 1, &probe_log);
    write_fake_engine_binary_with_probe_log(&fake_bin_dir, "podman", 1, &probe_log);

    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "macos".to_string());

    let remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_engine_unavailable".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo should-not-run >> '{}'", marker_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container-down".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized {
                image: "tak/test:v1".to_string(),
            }),
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("run must fail when no supported container engine is available");
    let message = format!("{err:#}");
    assert!(
        message.contains("infra error"),
        "error should preserve infra classification: {message}"
    );
    assert!(
        message.contains("remote-container-down"),
        "error should include strict target node id: {message}"
    );
    assert!(
        message.contains("no container engine available"),
        "error should include explicit engine availability reason: {message}"
    );
    assert!(
        message.contains("attempted probes: docker, podman"),
        "error should include deterministic attempted probe ordering: {message}"
    );
    assert!(
        !marker_file.exists(),
        "task command should not execute when engine resolution fails"
    );

    let probes = fs::read_to_string(&probe_log).expect("probe log should exist");
    assert_eq!(
        probes.lines().collect::<Vec<_>>(),
        vec!["docker", "podman"],
        "failed macos probe path should still preserve docker-then-podman order"
    );
}

/// Verifies strict remote mode surfaces explicit container lifecycle failure diagnostics.
#[tokio::test]
async fn remote_container_runtime_strict_lifecycle_failure_returns_infra_error() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 0);
    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "other".to_string());
    let _lifecycle_guard = ScopedEnvVar::set(
        "TAK_TEST_CONTAINER_LIFECYCLE_FAILURES",
        "remote-container-strict:pull".to_string(),
    );

    let remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_lifecycle_strict".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo should-not-run >> '{}'", marker_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-container-strict".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: Some(RemoteRuntimeSpec::Containerized {
                image: "tak/test:v1".to_string(),
            }),
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("strict lifecycle failure should surface an infra error");
    let message = format!("{err:#}");
    assert!(
        message.contains("infra error"),
        "error should preserve infra classification: {message}"
    );
    assert!(
        message.contains("remote-container-strict"),
        "error should include strict remote target id: {message}"
    );
    assert!(
        message.contains("container lifecycle pull failed"),
        "error should include explicit lifecycle stage diagnostics: {message}"
    );
    assert!(
        !marker_file.exists(),
        "task command should not execute when strict container lifecycle fails"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status"],
        "strict lifecycle failure should stop before submit/events/result"
    );
}

/// Verifies ordered fallback advances when the first node has a container lifecycle failure.
#[tokio::test]
async fn remote_container_runtime_fallback_advances_on_first_lifecycle_failure() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 0);
    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "other".to_string());
    let _lifecycle_guard = ScopedEnvVar::set(
        "TAK_TEST_CONTAINER_LIFECYCLE_FAILURES",
        "remote-container-a:start".to_string(),
    );

    let first_remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );
    let second_remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_lifecycle_fallback".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "echo node=$TAK_REMOTE_ENGINE runtime=$TAK_REMOTE_RUNTIME >> '{}'",
                    marker_file.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(vec![
            RemoteSpec {
                id: "remote-container-a".to_string(),
                endpoint: Some(first_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: Some(RemoteRuntimeSpec::Containerized {
                    image: "tak/test:v1".to_string(),
                }),
            },
            RemoteSpec {
                id: "remote-container-b".to_string(),
                endpoint: Some(second_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: Some(RemoteRuntimeSpec::Containerized {
                    image: "tak/test:v1".to_string(),
                }),
            },
        ])),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("fallback should advance to second node after first lifecycle failure");
    let result = summary
        .results
        .get(&label)
        .expect("summary should contain result");
    assert!(result.success);
    assert_eq!(result.remote_node_id.as_deref(), Some("remote-container-b"));
    assert_eq!(result.remote_runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.remote_runtime_engine.as_deref(), Some("docker"));

    let marker = fs::read_to_string(&marker_file).expect("runtime marker should be written");
    assert!(
        marker.contains("runtime=containerized"),
        "fallback node should still execute with containerized runtime metadata: {marker}"
    );
    assert!(
        marker.contains("node=docker"),
        "fallback node should preserve resolved engine metadata: {marker}"
    );

    assert_eq!(
        first_remote.call_order(),
        vec!["capabilities", "status"],
        "first lifecycle-failing node should not reach submit/events/result"
    );
    assert_eq!(
        second_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "fallback node should complete full remote protocol lifecycle"
    );
}

/// Verifies ordered fallback keeps infra failure semantics when all container candidates fail.
#[tokio::test]
async fn remote_container_runtime_all_candidates_fail_without_local_fallback() {
    let _env_lock = env_var_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 0);
    let _path_guard = ScopedEnvVar::set("PATH", prepend_path(&fake_bin_dir));
    let _platform_guard = ScopedEnvVar::set("TAK_TEST_HOST_PLATFORM", "other".to_string());
    let _lifecycle_guard = ScopedEnvVar::set(
        "TAK_TEST_CONTAINER_LIFECYCLE_FAILURES",
        "remote-container-a:pull,remote-container-b:runtime".to_string(),
    );

    let first_remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );
    let second_remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "container_lifecycle_all_fail".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo should-not-run >> '{}'", marker_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(vec![
            RemoteSpec {
                id: "remote-container-a".to_string(),
                endpoint: Some(first_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: Some(RemoteRuntimeSpec::Containerized {
                    image: "tak/test:v1".to_string(),
                }),
            },
            RemoteSpec {
                id: "remote-container-b".to_string(),
                endpoint: Some(second_remote.endpoint()),
                transport_kind: RemoteTransportKind::DirectHttps,
                service_auth_env: None,
                runtime: Some(RemoteRuntimeSpec::Containerized {
                    image: "tak/test:v1".to_string(),
                }),
            },
        ])),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("all lifecycle-failing candidates should return infra failure");
    let message = format!("{err:#}");
    assert!(
        message.contains("infra error: no reachable remote fallback candidates"),
        "all-candidate failure should preserve remote fallback infra semantics: {message}"
    );
    assert!(
        message.contains("container lifecycle pull failed"),
        "diagnostics should preserve pull-stage lifecycle reason: {message}"
    );
    assert!(
        message.contains("container lifecycle runtime failed"),
        "diagnostics should preserve runtime-stage lifecycle reason: {message}"
    );
    assert!(
        !marker_file.exists(),
        "task command should never run when all remote candidates fail lifecycle"
    );
    assert_eq!(
        first_remote.call_order(),
        vec!["capabilities", "status"],
        "first failing candidate should stop before submit/events/result"
    );
    assert_eq!(
        second_remote.call_order(),
        vec!["capabilities", "status"],
        "second failing candidate should also stop before submit/events/result"
    );
}

/// Verifies remote log chunks are persisted in sequence order and result output metadata is stored.
#[tokio::test]
async fn remote_only_single_persists_ordered_log_chunks_and_output_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteStreamingServer::spawn(
        vec![
            r#"{"events":[{"seq":2,"kind":"TASK_LOG_CHUNK","chunk":"second\n"},{"seq":1,"kind":"TASK_LOG_CHUNK","chunk":"first\n"}],"done":false}"#.to_string(),
            r#"{"events":[{"seq":3,"kind":"TASK_LOG_CHUNK","chunk":"third\n"}],"done":true}"#
                .to_string(),
        ],
        r#"{
          "success": true,
          "exit_code": 0,
          "sync_mode": "OUTPUTS_AND_LOGS",
          "outputs": [
            {"path":"dist/app.bin","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":11},
            {"path":"reports/test.xml","digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":29}
          ]
        }"#
        .to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_stream_and_sync".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");
    let result = summary
        .results
        .get(&label)
        .expect("summary should contain task result");

    let logged = result
        .remote_logs
        .iter()
        .map(|chunk| (chunk.seq, chunk.chunk.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        logged,
        vec![(1, "first\n"), (2, "second\n"), (3, "third\n")],
        "remote logs should be persisted once in checkpoint order"
    );

    let outputs = result
        .synced_outputs
        .iter()
        .map(|output| {
            (
                output.path.as_str(),
                output.digest.as_str(),
                output.size_bytes,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outputs,
        vec![
            (
                "dist/app.bin",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                11
            ),
            (
                "reports/test.xml",
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                29
            )
        ],
        "result envelope outputs should be persisted with path/digest/size"
    );
    assert_eq!(
        remote.call_order(),
        vec![
            "capabilities",
            "status",
            "submit",
            "events",
            "events",
            "result",
        ],
        "executor should keep protocol ordering while polling event stream"
    );
}

/// Verifies V1 rejects remote result sync modes other than OUTPUTS_AND_LOGS.
#[tokio::test]
async fn remote_only_single_rejects_unsupported_result_sync_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = FakeRemoteStreamingServer::spawn(
        vec![r#"{"events":[],"done":true}"#.to_string()],
        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_ONLY","outputs":[]}"#.to_string(),
    );

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_bad_sync_mode".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-primary".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("unsupported sync mode must fail in V1 remote flow");
    let message = err.to_string();
    assert!(
        message.contains("OUTPUTS_AND_LOGS"),
        "error should name supported sync mode: {message}"
    );
    assert!(
        message.contains("OUTPUTS_ONLY"),
        "error should include unsupported sync mode value: {message}"
    );
}

/// Verifies direct HTTPS and Tor transports follow the same remote protocol lifecycle contract.
#[tokio::test]
async fn direct_and_tor_transports_share_remote_protocol_contract() {
    let direct_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });
    let tor_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let direct_temp = tempfile::tempdir().expect("direct tempdir");
    let direct_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "direct_transport".to_string(),
    };
    let direct_task = ResolvedTask {
        label: direct_label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-direct".to_string(),
            endpoint: Some(direct_remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };
    let mut direct_tasks = BTreeMap::new();
    direct_tasks.insert(direct_label.clone(), direct_task);
    let direct_spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: direct_temp.path().to_path_buf(),
        tasks: direct_tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let direct_summary = run_tasks(
        &direct_spec,
        std::slice::from_ref(&direct_label),
        &RunOptions::default(),
    )
    .await
    .expect("direct transport run should succeed");
    let direct_result = direct_summary
        .results
        .get(&direct_label)
        .expect("direct summary contains result");

    let tor_temp = tempfile::tempdir().expect("tor tempdir");
    let tor_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "tor_transport".to_string(),
    };
    let tor_task = ResolvedTask {
        label: tor_label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-tor".to_string(),
            endpoint: Some(tor_remote.endpoint()),
            transport_kind: RemoteTransportKind::Tor,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };
    let mut tor_tasks = BTreeMap::new();
    tor_tasks.insert(tor_label.clone(), tor_task);
    let tor_spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: tor_temp.path().to_path_buf(),
        tasks: tor_tasks,
        limiters: HashMap::<LimiterKey, tak_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let tor_summary = run_tasks(
        &tor_spec,
        std::slice::from_ref(&tor_label),
        &RunOptions::default(),
    )
    .await
    .expect("tor transport run should succeed");
    let tor_result = tor_summary
        .results
        .get(&tor_label)
        .expect("tor summary contains result");

    assert_eq!(
        direct_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "direct transport should keep canonical protocol sequence"
    );
    assert_eq!(
        tor_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "tor transport should keep canonical protocol sequence"
    );
    assert_eq!(
        direct_result.success, tor_result.success,
        "direct and tor transports should surface the same terminal success status"
    );
    assert_eq!(
        direct_result.exit_code, tor_result.exit_code,
        "direct and tor transports should surface the same terminal exit code"
    );
    assert_eq!(
        direct_result.placement_mode,
        PlacementMode::Remote,
        "direct transport should preserve remote placement metadata"
    );
    assert_eq!(
        tor_result.placement_mode,
        PlacementMode::Remote,
        "tor transport should preserve remote placement metadata"
    );
    assert_eq!(
        direct_result.remote_transport_kind.as_deref(),
        Some("direct"),
        "direct transport result should persist direct transport kind metadata"
    );
    assert_eq!(
        tor_result.remote_transport_kind.as_deref(),
        Some("tor"),
        "tor transport result should persist tor transport kind metadata"
    );
}
