//! CLI contract tests for user-visible command behavior.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use tak_core::model::Scope;
use takd::{new_shared_manager_with_db, run_server};

/// Writes a `TASKS.py` file under `apps/web` for command-level tests.
fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root.join("apps/web")).expect("mkdir");
    fs::write(root.join("apps/web/TASKS.py"), body).expect("write tasks");
}

/// Strips ANSI escape sequences for stable assertions.
fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_escape = false;
    for ch in input.chars() {
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        if ch == '\u{1b}' {
            in_escape = true;
            continue;
        }
        result.push(ch);
    }
    result
}

/// Extracts one key field value from a `tak run` summary line for a specific task label.
fn extract_summary_field(summary: &str, label: &str, field: &str) -> Option<String> {
    let marker = format!("{field}=");
    let prefix = format!("{label}:");
    let line = summary
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))?;
    let start = line.find(&marker)? + marker.len();
    let tail = &line[start..];
    let end = tail.find([',', ')']).unwrap_or(tail.len());
    Some(tail[..end].trim().to_string())
}

fn prepend_path(prefix_dir: &std::path::Path) -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    if current_path.is_empty() {
        prefix_dir.display().to_string()
    } else {
        format!("{}:{current_path}", prefix_dir.display())
    }
}

#[cfg(unix)]
fn write_fake_engine_binary(
    bin_dir: &std::path::Path,
    binary_name: &str,
    exit_code: i32,
    probe_log_path: &std::path::Path,
) {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(bin_dir).expect("mkdir fake engine bin dir");
    let binary_path = bin_dir.join(binary_name);
    let script = format!(
        "#!/bin/sh\necho {name} >> \"{log}\"\nexit {code}\n",
        name = binary_name,
        log = probe_log_path.display(),
        code = exit_code
    );
    fs::write(&binary_path, script).expect("write fake engine binary");
    let mut perms = fs::metadata(&binary_path)
        .expect("stat fake engine binary")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary_path, perms).expect("chmod fake engine binary");
}

#[cfg(not(unix))]
fn write_fake_engine_binary(
    _bin_dir: &std::path::Path,
    _binary_name: &str,
    _exit_code: i32,
    _probe_log_path: &std::path::Path,
) {
    panic!("fake engine binary helper currently supports unix only");
}

#[derive(Clone, Copy, Debug)]
struct FakeRemoteProtocolConfig {
    preflight_compatible: bool,
    result_success: bool,
    result_exit_code: i32,
}

struct FakeRemoteProtocolServer {
    port: u16,
    calls: Arc<Mutex<Vec<String>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl FakeRemoteProtocolServer {
    fn spawn(config: FakeRemoteProtocolConfig) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake protocol remote");
        let port = listener.local_addr().expect("fake protocol addr").port();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_for_thread = Arc::clone(&calls);

        let handle = std::thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept fake protocol request");
                let request_line = read_http_request_line(&mut stream);
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();

                let response = if path == "/__shutdown" {
                    Some((
                        "200 OK",
                        r#"{"shutdown":true}"#.to_string(),
                        Some("shutdown"),
                    ))
                } else if path.starts_with("/v1/preflight") {
                    Some((
                        "200 OK",
                        format!(r#"{{"compatible":{}}}"#, config.preflight_compatible),
                        Some("preflight"),
                    ))
                } else if path == "/v1/node/capabilities" {
                    Some((
                        "200 OK",
                        format!(r#"{{"compatible":{}}}"#, config.preflight_compatible),
                        Some("capabilities"),
                    ))
                } else if path == "/v1/node/status" {
                    Some(("200 OK", r#"{"healthy":true}"#.to_string(), Some("status")))
                } else if path.starts_with("/v1/submit") || path == "/v1/tasks/submit" {
                    Some(("200 OK", r#"{"accepted":true}"#.to_string(), Some("submit")))
                } else if path.starts_with("/v1/events")
                    || (path.starts_with("/v1/tasks/") && path.contains("/events"))
                {
                    Some(("200 OK", r#"{"events":[]}"#.to_string(), Some("events")))
                } else if path.starts_with("/v1/result")
                    || (path.starts_with("/v1/tasks/") && path.ends_with("/result"))
                {
                    Some((
                        "200 OK",
                        format!(
                            r#"{{"success":{},"exit_code":{}}}"#,
                            config.result_success, config.result_exit_code
                        ),
                        Some("result"),
                    ))
                } else {
                    Some((
                        "404 Not Found",
                        r#"{"error":"not found"}"#.to_string(),
                        None,
                    ))
                };

                if let Some((status, body, marker)) = response {
                    if let Some(marker) = marker {
                        calls_for_thread
                            .lock()
                            .expect("lock protocol calls")
                            .push(marker.to_string());
                    }
                    write_http_json_response(&mut stream, status, &body);
                    if path == "/__shutdown" {
                        break;
                    }
                }
            }
        });

        Self {
            port,
            calls,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.calls.lock().expect("lock protocol calls").clone()
    }
}

impl Drop for FakeRemoteProtocolServer {
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

struct FakeRemoteResumableEventsServer {
    port: u16,
    calls: Arc<Mutex<Vec<String>>>,
    after_seq_calls: Arc<Mutex<Vec<u64>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl FakeRemoteResumableEventsServer {
    fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake resumable remote");
        let port = listener.local_addr().expect("fake resumable addr").port();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let after_seq_calls = Arc::new(Mutex::new(Vec::new()));
        let calls_for_thread = Arc::clone(&calls);
        let after_seq_for_thread = Arc::clone(&after_seq_calls);

        let handle = std::thread::spawn(move || {
            let mut events_request_count = 0_usize;

            loop {
                let (mut stream, _) = listener.accept().expect("accept fake resumable request");
                let request_line = read_http_request_line(&mut stream);
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();

                if path == "/__shutdown" {
                    write_http_json_response(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }

                if path.starts_with("/v1/preflight") {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
                        .push("preflight".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/capabilities" {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
                        .push("capabilities".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"compatible":true}"#);
                    continue;
                }

                if path == "/v1/node/status" {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
                        .push("status".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }

                if path.starts_with("/v1/submit") || path == "/v1/tasks/submit" {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
                        .push("submit".to_string());
                    write_http_json_response(&mut stream, "200 OK", r#"{"accepted":true}"#);
                    continue;
                }

                if path.starts_with("/v1/events")
                    || (path.starts_with("/v1/tasks/") && path.contains("/events"))
                {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
                        .push("events".to_string());

                    let after_seq = extract_query_u64(&path, "after_seq").unwrap_or(0);
                    after_seq_for_thread
                        .lock()
                        .expect("lock after_seq calls")
                        .push(after_seq);

                    events_request_count += 1;
                    match events_request_count {
                        1 => {
                            write_http_json_response(
                                &mut stream,
                                "200 OK",
                                r#"{"events":[{"seq":1,"kind":"TASK_LOG_CHUNK"},{"seq":2,"kind":"TASK_LOG_CHUNK"}],"done":false}"#,
                            );
                        }
                        2 => {
                            // Simulate mid-stream disconnect before a response payload is sent.
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                        3 => {
                            write_http_json_response(
                                &mut stream,
                                "200 OK",
                                r#"{"events":[{"seq":2,"kind":"TASK_LOG_CHUNK"},{"seq":1,"kind":"TASK_LOG_CHUNK"}],"done":false}"#,
                            );
                        }
                        4 => {
                            write_http_json_response(
                                &mut stream,
                                "200 OK",
                                r#"{"events":[{"seq":3,"kind":"TASK_LOG_CHUNK"}],"done":true}"#,
                            );
                        }
                        _ => {
                            write_http_json_response(
                                &mut stream,
                                "200 OK",
                                r#"{"events":[],"done":true}"#,
                            );
                        }
                    }
                    continue;
                }

                if path.starts_with("/v1/result")
                    || (path.starts_with("/v1/tasks/") && path.ends_with("/result"))
                {
                    calls_for_thread
                        .lock()
                        .expect("lock resumable protocol calls")
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
            calls,
            after_seq_calls,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn call_order(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("lock resumable protocol calls")
            .clone()
    }

    fn after_seq_calls(&self) -> Vec<u64> {
        self.after_seq_calls
            .lock()
            .expect("lock after_seq calls")
            .clone()
    }
}

impl Drop for FakeRemoteResumableEventsServer {
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

fn extract_query_u64(path: &str, key: &str) -> Option<u64> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            value.parse::<u64>().ok()
        } else {
            None
        }
    })
}

fn read_http_request_line(stream: &mut TcpStream) -> String {
    let mut request_bytes = Vec::new();
    let mut chunk = [0_u8; 512];
    loop {
        let read = stream.read(&mut chunk).expect("read request bytes");
        if read == 0 {
            break;
        }
        request_bytes.extend_from_slice(&chunk[..read]);
        if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8_lossy(&request_bytes)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

fn write_http_json_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write fake protocol response");
}

/// Verifies `tak list` prints canonical labels and bracketed dependencies.
#[test]
fn list_displays_canonical_dependency_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=build, steps=[cmd("echo", "ok")])
package = task("package", deps=[test], steps=[cmd("echo", "ok")])
SPEC = module_spec(tasks=[build, test, package])
SPEC
"#,
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path()).arg("list");
    let output = cmd.output().expect("list should execute");
    assert!(output.status.success(), "list must succeed");
    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);

    assert!(
        stdout.contains("//apps/web:build"),
        "missing canonical task label"
    );
    assert!(
        stdout.contains("//apps/web:test [//apps/web:build]"),
        "missing bracketed dependency output"
    );
    assert!(
        stdout.contains("//apps/web:package [//apps/web:test]"),
        "missing second dependency line"
    );
}

/// Verifies `tak list` includes ANSI colors for task names and dependency elements.
#[test]
fn list_uses_colors_for_elements() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=build, steps=[cmd("echo", "ok")])
SPEC = module_spec(tasks=[build, test])
SPEC
"#,
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path()).arg("list");
    let output = cmd.output().expect("list should execute");
    assert!(output.status.success(), "list must succeed");
    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_raw.contains('\u{1b}'),
        "list should include ANSI color sequences"
    );
}

/// Verifies `tak tree` shows hierarchy using tree glyphs.
#[test]
fn tree_renders_hierarchy() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=build, steps=[cmd("echo", "ok")])
package = task("package", deps=[test], steps=[cmd("echo", "ok")])
SPEC = module_spec(tasks=[build, test, package])
SPEC
"#,
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path()).arg("tree");
    let output = cmd.output().expect("tree should execute");
    assert!(output.status.success(), "tree must succeed");
    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(stdout.contains("Tak Tree"), "tree should include title");
    assert!(
        stdout.contains("└─"),
        "tree should include hierarchy glyphs"
    );
    assert!(
        stdout.contains("//apps/web:build"),
        "tree should include canonical labels"
    );
}

/// Verifies `tak explain` prints dependency information for a target.
#[test]
fn explain_shows_dependencies() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
SPEC = module_spec(tasks=[
  task("build", steps=[cmd("echo", "ok")]),
  task("test", deps=[":build"], steps=[cmd("echo", "ok")])
])
SPEC
"#,
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["explain", "apps/web:test"]);
    cmd.assert()
        .success()
        .stdout(contains("deps"))
        .stdout(contains("apps/web:build"));
}

/// Verifies `tak run` executes dependencies before the requested target.
#[test]
fn run_executes_target_and_dependencies() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    write_tasks(
        temp.path(),
        &format!(
            r#"
SPEC = module_spec(tasks=[
  task("build", steps=[cmd("sh", "-c", "echo build >> {log}")]),
  task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo test >> {log}")])
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path()).args(["run", "apps/web:test"]);
    cmd.assert().success();

    let output = fs::read_to_string(log_file).expect("log should exist");
    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["build", "test"]);
}

/// Verifies `LocalOnly(Local)` tasks run locally and report local placement metadata.
#[test]
fn run_local_only_execution_reports_local_placement_and_ignores_unused_remote_defs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
LOCAL = Local(id="dev-local", max_parallel_tasks=2)
UNUSED_REMOTE = Remote(id="unreachable-remote", endpoint="http://127.0.0.1:9")

SPEC = module_spec(tasks=[
  task(
    "local_only",
    steps=[cmd("sh", "-c", "echo local_only >> {log}")],
    execution=LocalOnly(LOCAL),
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:local_only"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=local"),
        "run output should include local placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=none"),
        "run output should include explicit empty remote node marker: {stdout}"
    );

    let execution_log = fs::read_to_string(log_file).expect("local log should exist");
    assert_eq!(
        execution_log.lines().collect::<Vec<_>>(),
        vec!["local_only"]
    );
}

/// Verifies strict pinned `RemoteOnly(Remote)` succeeds on a healthy endpoint and reports node placement.
#[test]
fn run_remote_only_single_healthy_endpoint_reports_remote_placement() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-primary", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_only",
    steps=[cmd("sh", "-c", "echo remote_only >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            endpoint = remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_only"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-primary"),
        "run output should include selected strict remote node marker: {stdout}"
    );

    assert!(
        !log_file.exists(),
        "strict RemoteOnly V1 path should not execute task command locally"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "healthy strict node should complete canonical V1 handshake flow"
    );
}

/// Verifies strict `RemoteOnly(Remote)` rejects reachable endpoints that do not implement V1 handshake.
#[test]
fn run_remote_only_single_legacy_reachable_endpoint_fails_without_local_fallback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let legacy_listener = TcpListener::bind("127.0.0.1:0").expect("bind legacy reachable endpoint");
    let legacy_port = legacy_listener
        .local_addr()
        .expect("legacy listener addr")
        .port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-legacy", endpoint="http://127.0.0.1:{port}")

SPEC = module_spec(tasks=[
  task(
    "remote_legacy",
    steps=[cmd("sh", "-c", "echo should_not_run >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            port = legacy_port,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_legacy"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail when strict remote endpoint is reachable but lacks V1 handshake support"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("infra error"),
        "stderr should preserve infra classification: {stderr}"
    );
    assert!(
        stderr.contains("does not support V1"),
        "stderr should include explicit unsupported-remote-protocol reason: {stderr}"
    );
    assert!(
        !log_file.exists(),
        "strict RemoteOnly task must not execute command locally for legacy endpoints"
    );
}

/// Verifies strict pinned `RemoteOnly(Remote)` fails as infra error when node is unavailable.
#[test]
fn run_remote_only_single_unavailable_endpoint_fails_without_local_fallback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-down", endpoint="http://127.0.0.1:9")

SPEC = module_spec(tasks=[
  task(
    "remote_only_down",
    steps=[cmd("sh", "-c", "echo should_not_run >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_only_down"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail when strict remote node is unavailable"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("infra error"),
        "stderr should report infra-class failure: {stderr}"
    );
    assert!(
        stderr.contains("unavailable at"),
        "stderr should include explicit remote availability reason: {stderr}"
    );
    assert!(
        stderr.contains("remote-down"),
        "stderr should include strict remote node id: {stderr}"
    );
    assert!(
        !stderr.contains("no reachable remote fallback candidates"),
        "strict pin failure should not use ordered-fallback error path: {stderr}"
    );

    assert!(
        !log_file.exists(),
        "task should not run locally when strict remote node is unavailable"
    );
}

/// Verifies `RemoteOnly([Remote...])` falls back in listed order and succeeds on the first reachable node.
#[test]
fn run_remote_only_list_falls_back_in_order_to_first_reachable_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let fallback_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE_A = Remote(id="remote-a", endpoint="http://127.0.0.1:9")
REMOTE_B = Remote(id="remote-b", endpoint="{fallback_endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_list",
    steps=[cmd("sh", "-c", "echo remote_list >> {log}")],
    execution=RemoteOnly([REMOTE_A, REMOTE_B]),
  )
])
SPEC
"#,
            fallback_endpoint = fallback_remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_list"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed with fallback candidate\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-b"),
        "run output should include first reachable node id from ordered list: {stdout}"
    );

    assert!(
        !log_file.exists(),
        "RemoteOnly fallback should delegate execution and avoid local command execution"
    );
    assert_eq!(
        fallback_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "fallback node should complete canonical V1 handshake flow"
    );
}

/// Verifies `RemoteOnly([Remote...])` does not probe later candidates after the first submit-capable node.
#[test]
fn run_remote_only_list_stops_after_first_reachable_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let first_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE_A = Remote(id="remote-a", endpoint="{first_endpoint}")
REMOTE_B = Remote(id="remote-b", endpoint="::invalid::endpoint::")

SPEC = module_spec(tasks=[
  task(
    "remote_list_first_wins",
    steps=[cmd("sh", "-c", "echo remote_list_first_wins >> {log}")],
    execution=RemoteOnly([REMOTE_A, REMOTE_B]),
  )
])
SPEC
"#,
            first_endpoint = first_remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_list_first_wins"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run should succeed on first reachable node without touching later invalid candidates\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("remote_node=remote-a"),
        "run output should report first candidate node id: {stdout}"
    );

    assert!(
        !log_file.exists(),
        "first submit-capable remote should run remotely with no local command execution"
    );
    assert_eq!(
        first_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "first submit-capable node should complete handshake and short-circuit later candidates"
    );
}

/// Verifies `RemoteOnly([Remote...])` fails with infra error when no candidate endpoint is reachable.
#[test]
fn run_remote_only_list_all_unavailable_returns_infra_error_without_local_fallback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE_A = Remote(id="remote-a", endpoint="http://127.0.0.1:9")
REMOTE_B = Remote(id="remote-b", endpoint="http://127.0.0.1:10")

SPEC = module_spec(tasks=[
  task(
    "remote_list_down",
    steps=[cmd("sh", "-c", "echo should_not_run >> {log}")],
    execution=RemoteOnly([REMOTE_A, REMOTE_B]),
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_list_down"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail when all ordered remote candidates are unavailable"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("infra error"),
        "stderr should report infra-class failure: {stderr}"
    );
    assert!(
        stderr.contains("remote-a"),
        "stderr should include first remote node id: {stderr}"
    );
    assert!(
        stderr.contains("remote-b"),
        "stderr should include second remote node id: {stderr}"
    );

    assert!(
        !log_file.exists(),
        "task should not run locally when all remote fallback candidates are unavailable"
    );
}

/// Verifies `ByCustomPolicy(policy_fn)` only exposes V1 policy context fields and reports reason.
#[test]
fn run_by_custom_policy_local_decision_uses_v1_context_surface_and_reports_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
POLICY_CONTEXT = PolicyContext(
  task_side_effecting=True,
  local_cpu_percent=11.0,
  remotes={{}},
  remote_any_reachable=False,
)

def choose_runtime(ctx):
    _ = ctx["task"]["side_effecting"]
    _ = ctx["local"]["cpu_percent"]
    _ = ctx["remote_any_reachable"]
    _ = policy_remote(ctx, "missing-node")

    has_extra = "forbidden_extra_field" in ctx

    if has_extra:
        return Decision_local(reason="unexpected_context_surface")
    return Decision_local(reason=REASON_SIDE_EFFECTING_TASK)

SPEC = module_spec(tasks=[
  task(
    "policy_local",
    steps=[cmd("sh", "-c", "echo policy_local >> {log}")],
    execution=ByCustomPolicy(choose_runtime),
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:policy_local"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=local"),
        "run output should include local placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=none"),
        "run output should include explicit empty remote node marker: {stdout}"
    );
    assert!(
        stdout.contains("reason=SIDE_EFFECTING_TASK"),
        "run output should include policy decision reason: {stdout}"
    );

    let execution_log = fs::read_to_string(log_file).expect("task log should exist");
    assert_eq!(
        execution_log.lines().collect::<Vec<_>>(),
        vec!["policy_local"]
    );
}

/// Verifies strict `Decision.remote` policy output is surfaced with node + reason and remains stable across retries.
#[test]
fn run_by_custom_policy_remote_decision_reports_node_reason_and_stays_stable_for_retries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let retry_marker = temp.path().join("retry.marker");
    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
POLICY_CALLS = 0
POLICY_CONTEXT = PolicyContext(
  task_side_effecting=False,
  local_cpu_percent=95.0,
  remotes={{
    "remote-primary": RemoteRuntimeView(
      endpoint="http://127.0.0.1:{port}",
      healthy=True,
      queue_eta_s=1.0,
    )
  }},
  remote_any_reachable=True,
)

def choose_runtime(ctx):
    global POLICY_CALLS
    POLICY_CALLS += 1
    if POLICY_CALLS > 1:
        return Decision_local(reason="policy_mutated")

    remote = policy_remote(ctx, "remote-primary")
    if (
      ctx["local"]["cpu_percent"] >= 85
      and remote
      and remote["healthy"]
      and remote["queue_eta_s"] < 20
    ):
        return Decision_remote("remote-primary", reason=REASON_LOCAL_CPU_HIGH_ARM_IDLE)
    return Decision_local(reason=REASON_DEFAULT_LOCAL_POLICY)

SPEC = module_spec(tasks=[
  task(
    "policy_remote_retry",
    retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)),
    steps=[cmd("sh", "-c", "if [ -f {marker} ]; then echo policy_remote_retry >> {log}; exit 0; else touch {marker}; exit 42; fi")],
    execution=ByCustomPolicy(choose_runtime),
  )
])
SPEC
"#,
            port = remote_port,
            marker = retry_marker.display(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:policy_remote_retry"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("attempts=2"),
        "run output should show retry count: {stdout}"
    );
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-primary"),
        "run output should include selected remote node marker: {stdout}"
    );
    assert!(
        stdout.contains("reason=LOCAL_CPU_HIGH_ARM_IDLE"),
        "run output should include stable policy reason marker: {stdout}"
    );
    assert!(
        !stdout.contains("reason=policy_mutated"),
        "policy decision should not mutate during retries: {stdout}"
    );

    let execution_log = fs::read_to_string(log_file).expect("task log should exist");
    assert_eq!(
        execution_log.lines().collect::<Vec<_>>(),
        vec!["policy_remote_retry"]
    );
}

/// Verifies policy can choose local with explicit unreachable-remote reason.
#[test]
fn run_by_custom_policy_remote_any_unreachable_chooses_local_with_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
POLICY_CONTEXT = PolicyContext(
  task_side_effecting=False,
  local_cpu_percent=93.0,
  remotes={{}},
  remote_any_reachable=False,
)

def choose_runtime(ctx):
    if not ctx["remote_any_reachable"]:
        return Decision_local(reason=REASON_NO_REMOTE_REACHABLE)
    return Decision_remote_any(["remote-a"], reason=REASON_LOCAL_CPU_HIGH)

SPEC = module_spec(tasks=[
  task(
    "policy_remote_any_down",
    steps=[cmd("sh", "-c", "echo policy_remote_any_down >> {log}")],
    execution=ByCustomPolicy(choose_runtime),
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:policy_remote_any_down"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=local"),
        "run output should include local placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=none"),
        "run output should include explicit empty remote node marker: {stdout}"
    );
    assert!(
        stdout.contains("reason=NO_REMOTE_REACHABLE"),
        "run output should include no-remote reason marker: {stdout}"
    );

    let execution_log = fs::read_to_string(log_file).expect("task log should exist");
    assert_eq!(
        execution_log.lines().collect::<Vec<_>>(),
        vec!["policy_remote_any_down"]
    );
}

/// Verifies named `ByCustomPolicy("...")` evaluates at runtime (no precompiled static decision).
#[test]
fn run_by_custom_policy_named_function_executes_runtime_policy_and_reports_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-primary", endpoint="http://127.0.0.1:{port}")
POLICY_CONTEXT = PolicyContext(
  task_side_effecting=False,
  local_cpu_percent=91.0,
  remotes={{
    "remote-primary": RemoteRuntimeView(
      endpoint="http://127.0.0.1:{port}",
      healthy=True,
      queue_eta_s=1.0,
    )
  }},
  remote_any_reachable=True,
)

def choose_runtime(ctx):
    if ctx["local"]["cpu_percent"] >= 85:
        return Decision_remote("remote-primary", reason=REASON_LOCAL_CPU_HIGH_ARM_IDLE)
    return Decision_local(reason=REASON_DEFAULT_LOCAL_POLICY)

SPEC = module_spec(tasks=[
  task(
    "policy_runtime_named",
    steps=[cmd("sh", "-c", "echo should_not_run_locally")],
    execution=ByCustomPolicy("choose_runtime"),
  )
])
SPEC
"#,
            port = remote_port
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:policy_runtime_named"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-primary"),
        "run output should include selected remote node marker: {stdout}"
    );
    assert!(
        stdout.contains("reason=LOCAL_CPU_HIGH_ARM_IDLE"),
        "run output should include runtime policy reason marker: {stdout}"
    );
}

/// Verifies `CurrentState` transfer boundary ordering is deterministic (`roots -> ignored -> include`).
#[test]
fn run_remote_only_current_state_boundary_is_deterministic() {
    let temp = tempfile::tempdir().expect("tempdir");
    let list_a = temp.path().join("manifest-a.txt");
    let list_b = temp.path().join("manifest-b.txt");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let project_dir = temp.path().join("apps/web/project");
    let ignored_dir = project_dir.join("ignored");
    fs::create_dir_all(&ignored_dir).expect("mkdir ignored dir");
    fs::create_dir_all(temp.path().join("apps/web/outside")).expect("mkdir outside dir");
    fs::write(project_dir.join("keep.txt"), "keep\n").expect("write keep");
    fs::write(ignored_dir.join("drop.txt"), "drop\n").expect("write drop");
    fs::write(ignored_dir.join("reinclude.txt"), "reinclude\n").expect("write reinclude");
    fs::write(
        temp.path().join("apps/web/outside/should_not_transfer.txt"),
        "outside\n",
    )
    .expect("write outside");

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-primary", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "context_a",
    steps=[cmd("sh", "-c", "find . -type f | LC_ALL=C sort > {list_a}")],
    context=CurrentState(
      roots=[path("//apps/web/project")],
      ignored=[path("//apps/web/project/ignored"), path("//apps/web/project/ignored")],
      include=[
        path("//apps/web/project/ignored/reinclude.txt"),
        path("//apps/web/outside/should_not_transfer.txt"),
      ],
    ),
    execution=RemoteOnly(REMOTE),
  ),
  task(
    "context_b",
    steps=[cmd("sh", "-c", "find . -type f | LC_ALL=C sort > {list_b}")],
    context=CurrentState(
      roots=[path("//apps/web/./project")],
      ignored=[path("//apps/web/project/ignored")],
      include=[path("//apps/web/project/ignored/reinclude.txt"), path("//apps/web/project/ignored/reinclude.txt")],
    ),
    execution=RemoteOnly(REMOTE),
  ),
])
SPEC
"#,
            endpoint = remote.endpoint(),
            list_a = list_a.display(),
            list_b = list_b.display(),
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:context_a", "apps/web:context_b"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !list_a.exists() && !list_b.exists(),
        "strict V1 remote path should not execute local marker commands while still producing deterministic context hashes"
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    let hash_a = extract_summary_field(&stdout, "apps/web:context_a", "context_hash")
        .expect("context_a summary should include context hash");
    let hash_b = extract_summary_field(&stdout, "apps/web:context_b", "context_hash")
        .expect("context_b summary should include context hash");
    assert_eq!(
        hash_a, hash_b,
        "semantically equivalent boundaries should produce stable ContextManifest hashes"
    );
    assert_eq!(
        remote.call_order(),
        vec![
            "capabilities",
            "status",
            "submit",
            "events",
            "result",
            "capabilities",
            "status",
            "submit",
            "events",
            "result"
        ],
        "each strict RemoteOnly task should run through canonical V1 handshake flow"
    );
}

/// Verifies remote protocol handshake order is
/// `capabilities -> status -> submit -> events -> result`.
#[test]
fn run_remote_only_handshake_follows_preflight_submit_events_result_order() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-handshake", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_handshake_ok",
    steps=[cmd("sh", "-c", "echo remote_handshake_ok >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            endpoint = remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_handshake_ok"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-handshake"),
        "run output should include selected remote node marker: {stdout}"
    );

    assert!(
        !log_file.exists(),
        "strict V1 remote handshake should not execute task steps locally"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "remote protocol order should follow capabilities->status->submit->events->result"
    );
}

/// Verifies preflight capability mismatch fails as infra error before submit.
#[test]
fn run_remote_only_handshake_preflight_mismatch_blocks_submit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: false,
        result_success: true,
        result_exit_code: 0,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-mismatch", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_preflight_mismatch",
    steps=[cmd("sh", "-c", "echo should_not_run >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            endpoint = remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_preflight_mismatch"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail on preflight capability mismatch"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("infra error"),
        "stderr should include infra error classification: {stderr}"
    );
    assert!(
        !log_file.exists(),
        "task should not run when preflight capability check fails"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities"],
        "preflight mismatch should block submit/events/result"
    );
}

/// Verifies terminal task status follows the remote result envelope after successful submit/events.
#[test]
fn run_remote_only_handshake_result_envelope_controls_terminal_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: false,
        result_exit_code: 42,
    });

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-result", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_result_failure",
    steps=[cmd("sh", "-c", "echo remote_result_failure >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            endpoint = remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_result_failure"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail when remote result envelope reports failure"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("task apps/web:remote_result_failure failed"),
        "stderr should report terminal task failure from result envelope: {stderr}"
    );

    assert!(
        !log_file.exists(),
        "strict V1 remote handshake should not execute task steps locally on result failure"
    );
    assert_eq!(
        remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "result should be fetched even when events stream has no log chunks"
    );
}

/// Verifies event stream resume uses `after_seq` checkpointing and ignores replayed older sequence values.
#[test]
fn run_remote_only_handshake_events_resume_uses_after_seq_without_duplicate_regression() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");
    let remote = FakeRemoteResumableEventsServer::spawn();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-resume", endpoint="{endpoint}")

SPEC = module_spec(tasks=[
  task(
    "remote_resume_ok",
    steps=[cmd("sh", "-c", "echo remote_resume_ok >> {log}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            endpoint = remote.endpoint(),
            log = log_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .args(["run", "apps/web:remote_resume_ok"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed after resumable events flow\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-resume"),
        "run output should include selected remote node marker: {stdout}"
    );

    assert!(
        !log_file.exists(),
        "strict V1 remote handshake should not execute task steps locally during resumable events"
    );
    assert_eq!(
        remote.after_seq_calls(),
        vec![0, 2, 2, 2],
        "events reconnect should resume from last checkpoint and ignore replayed older sequence values"
    );
    assert_eq!(
        remote.call_order(),
        vec![
            "capabilities",
            "status",
            "submit",
            "events",
            "events",
            "events",
            "events",
            "result"
        ],
        "events should retry and resume before terminal result fetch"
    );
}

/// Verifies strict remote container runtime selects Docker first and reports runtime placement fields.
#[test]
fn run_remote_only_container_runtime_uses_docker_and_reports_runtime_metadata() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 0, &probe_log);
    write_fake_engine_binary(&fake_bin_dir, "podman", 0, &probe_log);

    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(
  id="remote-container-docker",
  endpoint="http://127.0.0.1:{port}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)

SPEC = module_spec(tasks=[
  task(
    "container_runtime_docker",
    steps=[cmd("/bin/sh", "-c", "echo runtime=$TAK_REMOTE_RUNTIME engine=$TAK_REMOTE_ENGINE image=$TAK_REMOTE_CONTAINER_IMAGE >> {marker}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            port = remote_port,
            marker = marker_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .env("PATH", prepend_path(&fake_bin_dir))
        .env("TAK_TEST_HOST_PLATFORM", "other")
        .args(["run", "apps/web:container_runtime_docker"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let runtime_marker =
        fs::read_to_string(&marker_file).expect("runtime marker log should be created");
    assert!(
        runtime_marker.contains("runtime=containerized"),
        "task should observe containerized runtime marker: {runtime_marker}"
    );
    assert!(
        runtime_marker.contains("engine=docker"),
        "task should observe selected docker engine marker: {runtime_marker}"
    );
    assert!(
        runtime_marker.contains("image=tak/test:v1"),
        "task should observe configured container image marker: {runtime_marker}"
    );

    let probe_lines = fs::read_to_string(&probe_log).expect("probe log should exist");
    assert_eq!(
        probe_lines.lines().collect::<Vec<_>>(),
        vec!["docker"],
        "docker probe should short-circuit without probing podman"
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("placement=remote"),
        "run output should include remote placement marker: {stdout}"
    );
    assert!(
        stdout.contains("remote_node=remote-container-docker"),
        "run output should include selected remote node marker: {stdout}"
    );
    assert!(
        stdout.contains("runtime=containerized"),
        "run output should include runtime kind placement marker: {stdout}"
    );
    assert!(
        stdout.contains("runtime_engine=docker"),
        "run output should include selected engine placement marker: {stdout}"
    );
}

/// Verifies macOS container runtime engine fallback order is Docker first then Podman.
#[test]
fn run_remote_only_container_runtime_falls_back_to_podman_on_macos() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("runtime-marker.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 1, &probe_log);
    write_fake_engine_binary(&fake_bin_dir, "podman", 0, &probe_log);

    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(
  id="remote-container-podman",
  endpoint="http://127.0.0.1:{port}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)

SPEC = module_spec(tasks=[
  task(
    "container_runtime_podman",
    steps=[cmd("/bin/sh", "-c", "echo runtime=$TAK_REMOTE_RUNTIME engine=$TAK_REMOTE_ENGINE >> {marker}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            port = remote_port,
            marker = marker_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .env("PATH", prepend_path(&fake_bin_dir))
        .env("TAK_TEST_HOST_PLATFORM", "macos")
        .args(["run", "apps/web:container_runtime_podman"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed via podman fallback\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let probe_lines = fs::read_to_string(&probe_log).expect("probe log should exist");
    assert_eq!(
        probe_lines.lines().collect::<Vec<_>>(),
        vec!["docker", "podman"],
        "macos fallback should probe docker first, then podman"
    );

    let runtime_marker =
        fs::read_to_string(&marker_file).expect("runtime marker log should be created");
    assert!(
        runtime_marker.contains("runtime=containerized"),
        "task should observe containerized runtime marker: {runtime_marker}"
    );
    assert!(
        runtime_marker.contains("engine=podman"),
        "task should observe selected podman engine marker: {runtime_marker}"
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert!(
        stdout.contains("runtime=containerized"),
        "run output should include runtime kind placement marker: {stdout}"
    );
    assert!(
        stdout.contains("runtime_engine=podman"),
        "run output should include podman engine placement marker: {stdout}"
    );
}

/// Verifies strict pinned remote container runtime failures surface infra errors with no local fallback.
#[test]
fn run_remote_only_container_runtime_unavailable_is_infra_error_without_local_fallback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker_file = temp.path().join("should-not-run.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 1, &probe_log);

    let remote_listener = TcpListener::bind("127.0.0.1:0").expect("bind fake remote");
    let remote_port = remote_listener.local_addr().expect("listener addr").port();

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(
  id="remote-container-down",
  endpoint="http://127.0.0.1:{port}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)

SPEC = module_spec(tasks=[
  task(
    "container_runtime_unavailable",
    steps=[cmd("/bin/sh", "-c", "echo should_not_run >> {marker}")],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#,
            port = remote_port,
            marker = marker_file.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .env("PATH", fake_bin_dir.display().to_string())
        .env("TAK_TEST_HOST_PLATFORM", "other")
        .args(["run", "apps/web:container_runtime_unavailable"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        !output.status.success(),
        "run should fail when strict remote container engine is unavailable"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("infra error"),
        "stderr should classify runtime failure as infra error: {stderr}"
    );
    assert!(
        stderr.contains("remote-container-down"),
        "stderr should include strict remote node id: {stderr}"
    );
    assert!(
        stderr.contains("no container engine available"),
        "stderr should include explicit engine availability reason: {stderr}"
    );
    assert!(
        stderr.contains("attempted probes: docker"),
        "stderr should include attempted engine probe order: {stderr}"
    );
    assert!(
        !marker_file.exists(),
        "task must not run locally when strict remote container runtime cannot start"
    );
}

/// Verifies remote-focused BDD scenarios use a local multi-node cluster shape and deterministic engine policy.
#[test]
fn run_remote_multinode_containerized_gate_is_local_only_and_deterministic() {
    let temp = tempfile::tempdir().expect("tempdir");
    let execution_log = temp.path().join("run.log");
    let probe_log = temp.path().join("engine-probe.log");
    let fake_bin_dir = temp.path().join("fake-bin");
    write_fake_engine_binary(&fake_bin_dir, "docker", 1, &probe_log);
    write_fake_engine_binary(&fake_bin_dir, "podman", 0, &probe_log);

    let strict_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });
    let preflight_mismatch_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: false,
        result_success: true,
        result_exit_code: 0,
    });
    let fallback_remote = FakeRemoteProtocolServer::spawn(FakeRemoteProtocolConfig {
        preflight_compatible: true,
        result_success: true,
        result_exit_code: 0,
    });

    let strict_endpoint = strict_remote.endpoint();
    let mismatch_endpoint = preflight_mismatch_remote.endpoint();
    let fallback_endpoint = fallback_remote.endpoint();

    for endpoint in [&strict_endpoint, &mismatch_endpoint, &fallback_endpoint] {
        assert!(
            endpoint.starts_with("http://127.0.0.1:"),
            "remote cluster fixtures must stay loopback-local and offline: {endpoint}"
        );
    }

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE_STRICT = Remote(
  id="remote-strict",
  endpoint="{strict_endpoint}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)
REMOTE_MISMATCH = Remote(
  id="remote-mismatch",
  endpoint="{mismatch_endpoint}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)
REMOTE_FALLBACK = Remote(
  id="remote-fallback",
  endpoint="{fallback_endpoint}",
  runtime=ContainerRuntime(image="tak/test:v1"),
)

SPEC = module_spec(tasks=[
  task(
    "strict_first",
    steps=[cmd("sh", "-c", "echo strict_first >> {log}")],
    execution=RemoteOnly(REMOTE_STRICT),
  ),
  task(
    "fallback_second",
    deps=[":strict_first"],
    steps=[cmd("sh", "-c", "echo fallback_second >> {log}")],
    execution=RemoteOnly([REMOTE_MISMATCH, REMOTE_FALLBACK]),
  ),
])
SPEC
"#,
            strict_endpoint = strict_endpoint,
            mismatch_endpoint = mismatch_endpoint,
            fallback_endpoint = fallback_endpoint,
            log = execution_log.display()
        ),
    );

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .env("PATH", prepend_path(&fake_bin_dir))
        .env("TAK_TEST_HOST_PLATFORM", "macos")
        .args(["run", "apps/web:fallback_second"]);
    let output = cmd.output().expect("run should execute");
    assert!(
        output.status.success(),
        "run must succeed with local multi-node remote fixtures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = strip_ansi(&stdout_raw);
    assert_eq!(
        extract_summary_field(&stdout, "apps/web:strict_first", "placement").as_deref(),
        Some("remote"),
        "strict task should retain remote placement summary"
    );
    assert_eq!(
        extract_summary_field(&stdout, "apps/web:strict_first", "remote_node").as_deref(),
        Some("remote-strict"),
        "strict task should target the configured strict node"
    );
    assert_eq!(
        extract_summary_field(&stdout, "apps/web:strict_first", "runtime_engine").as_deref(),
        Some("podman"),
        "strict task should honor docker-first then podman fallback on macOS"
    );

    assert_eq!(
        extract_summary_field(&stdout, "apps/web:fallback_second", "placement").as_deref(),
        Some("remote"),
        "fallback task should retain remote placement summary"
    );
    assert_eq!(
        extract_summary_field(&stdout, "apps/web:fallback_second", "remote_node").as_deref(),
        Some("remote-fallback"),
        "fallback task should advance to the second remote node after mismatch"
    );
    assert_eq!(
        extract_summary_field(&stdout, "apps/web:fallback_second", "runtime_engine").as_deref(),
        Some("podman"),
        "fallback task should preserve deterministic engine fallback policy"
    );

    let execution_lines = fs::read_to_string(&execution_log).expect("execution log");
    assert_eq!(
        execution_lines.lines().collect::<Vec<_>>(),
        vec!["strict_first", "fallback_second"]
    );

    let probe_lines = fs::read_to_string(&probe_log).expect("probe log");
    assert_eq!(
        probe_lines.lines().collect::<Vec<_>>(),
        vec!["docker", "podman", "docker", "podman"],
        "both remote executions should apply docker-first and podman fallback deterministically"
    );

    assert_eq!(
        strict_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "strict node should complete full handshake flow"
    );
    assert_eq!(
        preflight_mismatch_remote.call_order(),
        vec!["capabilities"],
        "ordered fallback should exercise the first candidate before advancing"
    );
    assert_eq!(
        fallback_remote.call_order(),
        vec!["capabilities", "status", "submit", "events", "result"],
        "fallback node should complete full handshake after ordered selection"
    );
}

/// Verifies daemon-backed runs acquire and release leases for tasks with `needs`.
#[test]
fn run_with_needs_acquires_and_releases_daemon_lease() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket = temp.path().join("takd.sock");
    let db_path = temp.path().join("takd.sqlite");
    let log_file = temp.path().join("run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
SPEC = module_spec(tasks=[
  task(
    "limited",
    steps=[cmd("sh", "-c", "echo limited >> {log}")],
    needs=[need("cpu", 1, scope=MACHINE)]
  )
])
SPEC
"#,
            log = log_file.display()
        ),
    );

    let manager = new_shared_manager_with_db(db_path).expect("manager with sqlite");
    {
        let mut guard = manager.lock().expect("manager lock");
        guard.set_capacity("cpu", Scope::Machine, None, 8.0);
        guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    }

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let server_socket = socket.clone();
    let server_manager = Arc::clone(&manager);
    let server = runtime.spawn(async move {
        run_server(&server_socket, server_manager)
            .await
            .expect("daemon server should run");
    });

    let wait_deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() && Instant::now() < wait_deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(socket.exists(), "daemon socket should exist");

    let mut run_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    run_cmd
        .current_dir(temp.path())
        .env("TAKD_SOCKET", &socket)
        .args(["run", "apps/web:limited"]);
    run_cmd.assert().success();

    let mut status_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    status_cmd
        .current_dir(temp.path())
        .env("TAKD_SOCKET", &socket)
        .arg("status");
    status_cmd
        .assert()
        .success()
        .stdout(contains("active_leases: 0"));

    let output = fs::read_to_string(log_file).expect("log should exist");
    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["limited"]);

    server.abort();
    runtime.block_on(async {
        let _ = server.await;
    });
}

/// Fetches `/graph.json` over a raw local HTTP request for a given server port.
fn fetch_graph_json_http(port: u16) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream
        .write_all(b"GET /graph.json HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

/// Verifies `tak web` serves the graph UI without attempting browser-open in tests.
#[test]
fn web_serves_graph_and_prints_local_url_when_auto_open_is_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"
SPEC = module_spec(tasks=[
  task("build", steps=[cmd("echo", "ok")]),
  task("test", deps=[":build"], steps=[cmd("echo", "ok")])
])
SPEC
"#,
    );

    let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    child
        .current_dir(temp.path())
        .env("TAK_NO_BROWSER_OPEN", "1")
        .args(["web", "apps/web:test"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn().expect("web command should spawn");

    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut stdout_reader = BufReader::new(stdout);
    let mut url_line = String::new();
    stdout_reader
        .read_line(&mut url_line)
        .expect("should read url line");
    assert!(
        url_line.starts_with("web graph ui available at http://127.0.0.1:"),
        "unexpected first output line: {url_line:?}"
    );

    let url = url_line
        .trim_end()
        .strip_prefix("web graph ui available at ")
        .expect("url line should have expected prefix");
    let port = url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix('/'))
        .and_then(|value| value.parse::<u16>().ok())
        .expect("url should contain a valid port");

    let deadline = Instant::now() + Duration::from_secs(5);
    let response = loop {
        match fetch_graph_json_http(port) {
            Ok(response) => break response,
            Err(err) if Instant::now() < deadline => {
                let _ = err;
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => panic!("failed to query /graph.json before timeout: {err}"),
        }
    };
    assert!(response.contains("200 OK"), "response should be HTTP 200");
    assert!(
        response.contains("\"nodes\""),
        "graph response should include nodes payload"
    );

    let _ = child.kill();
    let _ = child.wait();

    let mut stderr_text = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut stderr_text);
    }
    assert!(
        stderr_text.contains("browser auto-open disabled"),
        "stderr should explain browser auto-open was disabled: {stderr_text}"
    );
}
