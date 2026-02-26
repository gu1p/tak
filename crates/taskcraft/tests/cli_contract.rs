//! CLI contract tests for user-visible command behavior.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command as StdCommand, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use taskcraft_core::model::Scope;
use taskcraftd::{new_shared_manager_with_db, run_server};

/// Writes a `TASKS.py` file under `apps/web` for command-level tests.
fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root.join("apps/web")).expect("mkdir");
    fs::write(root.join("apps/web/TASKS.py"), body).expect("write tasks");
}

/// Verifies `taskcraft list` outputs fully-qualified task labels.
#[test]
fn list_displays_fully_qualified_labels() {
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

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    cmd.current_dir(temp.path()).arg("list");
    cmd.assert()
        .success()
        .stdout(contains("//apps/web:build"))
        .stdout(contains("//apps/web:test"));
}

/// Verifies `taskcraft explain` prints dependency information for a target.
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

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    cmd.current_dir(temp.path())
        .args(["explain", "//apps/web:test"]);
    cmd.assert()
        .success()
        .stdout(contains("deps"))
        .stdout(contains("//apps/web:build"));
}

/// Verifies `taskcraft run` executes dependencies before the requested target.
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

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    cmd.current_dir(temp.path())
        .args(["run", "//apps/web:test"]);
    cmd.assert().success();

    let output = fs::read_to_string(log_file).expect("log should exist");
    assert_eq!(output.lines().collect::<Vec<_>>(), vec!["build", "test"]);
}

/// Verifies daemon-backed runs acquire and release leases for tasks with `needs`.
#[test]
fn run_with_needs_acquires_and_releases_daemon_lease() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket = temp.path().join("taskcraftd.sock");
    let db_path = temp.path().join("taskcraftd.sqlite");
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

    let mut run_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    run_cmd
        .current_dir(temp.path())
        .env("TASKCRAFTD_SOCKET", &socket)
        .args(["run", "//apps/web:limited"]);
    run_cmd.assert().success();

    let mut status_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    status_cmd
        .current_dir(temp.path())
        .env("TASKCRAFTD_SOCKET", &socket)
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

/// Verifies `taskcraft web` serves the graph UI without attempting browser-open in tests.
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

    let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin!("taskcraft"));
    child
        .current_dir(temp.path())
        .env("TASKCRAFT_NO_BROWSER_OPEN", "1")
        .args(["web", "//apps/web:test"])
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
