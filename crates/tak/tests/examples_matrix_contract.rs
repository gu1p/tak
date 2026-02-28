//! Exhaustive catalog-driven validation for all repository examples.
//!
//! This suite executes every example listed in `examples/catalog.toml` and verifies
//! loader/CLI/run behavior plus expected success/failure outcomes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tak_core::model::Scope;
use takd::{SubmitAttemptStore, new_shared_manager_with_db, run_remote_v1_http_server, run_server};

/// Represents one entry in `examples/catalog.toml`.
#[derive(Debug, Deserialize)]
struct ExampleCase {
    name: String,
    run_target: String,
    explain_target: String,
    expect_success: bool,
    requires_daemon: bool,
    #[serde(default)]
    remote_fixture: Option<RemoteFixtureKind>,
    #[serde(default)]
    expect_stdout_contains: Vec<String>,
    #[serde(default)]
    expect_stderr_contains: Vec<String>,
    check_files: Vec<String>,
    #[serde(default)]
    check_file_contains: Vec<CheckFileContainsExpectation>,
}

/// Optional deterministic remote fixture kind for catalog examples.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RemoteFixtureKind {
    DirectHttp,
    TorOnionHttp,
}

/// File content contract check for one catalog example.
#[derive(Debug, Deserialize)]
struct CheckFileContainsExpectation {
    path: String,
    contains: String,
}

/// Represents the full catalog structure in `examples/catalog.toml`.
#[derive(Debug, Deserialize)]
struct ExampleCatalog {
    example: Vec<ExampleCase>,
}

/// Validates every catalog example end-to-end through the CLI surface.
#[test]
fn validates_all_examples_from_catalog() {
    let repo_root = repo_root();
    let catalog = load_catalog(&repo_root.join("examples/catalog.toml"));

    for case in &catalog.example {
        run_catalog_case(&repo_root, case);
    }
}

/// Resolves the repository root from this crate's manifest directory.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above crate manifest")
        .to_path_buf()
}

/// Loads and parses the examples catalog TOML file.
fn load_catalog(path: &Path) -> ExampleCatalog {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed reading catalog {}: {err}", path.display()));
    toml::from_str(&content)
        .unwrap_or_else(|err| panic!("failed parsing catalog {}: {err}", path.display()))
}

/// Executes one catalog case in an isolated temp workspace copy.
fn run_catalog_case(repo_root: &Path, case: &ExampleCase) {
    let source_example = repo_root.join("examples").join(&case.name);
    assert!(
        source_example.exists(),
        "example path does not exist: {}",
        source_example.display()
    );

    let temp = tempfile::tempdir().expect("tempdir for example run");
    let working = temp.path().join("workspace");
    copy_dir_recursive(&source_example, &working);

    let remote_fixture = maybe_start_remote_fixture(case, temp.path());
    if let Some(fixture) = remote_fixture.as_ref() {
        rewrite_remote_endpoint_placeholders(&working, &fixture.endpoint);
    }

    let mut env_overrides = Vec::new();
    if let Some(fixture) = remote_fixture.as_ref() {
        env_overrides.extend_from_slice(&fixture.env_overrides);
    }

    let daemon = maybe_start_daemon(case, temp.path());

    run_tak_command(
        &working,
        daemon.as_ref().map(|d| d.socket.as_path()),
        &["list"],
        true,
        &env_overrides,
    );
    run_tak_command(
        &working,
        daemon.as_ref().map(|d| d.socket.as_path()),
        &["graph", &case.explain_target, "--format", "dot"],
        true,
        &env_overrides,
    );
    run_tak_command(
        &working,
        daemon.as_ref().map(|d| d.socket.as_path()),
        &["explain", &case.explain_target],
        true,
        &env_overrides,
    );
    let run_output = run_tak_command(
        &working,
        daemon.as_ref().map(|d| d.socket.as_path()),
        &["run", &case.run_target],
        case.expect_success,
        &env_overrides,
    );

    assert_expected_substrings(case, &run_output.stdout, &run_output.stderr);
    assert_expected_output_files(case, &working);

    for check in &case.check_file_contains {
        let output = working.join(&check.path);
        let body = fs::read_to_string(&output).unwrap_or_else(|err| {
            panic!(
                "failed reading expected file content for {} at {}: {err}",
                case.name,
                output.display()
            )
        });
        assert!(
            body.contains(&check.contains),
            "file content mismatch for {} at {}: expected to contain `{}`\nactual:\n{}",
            case.name,
            output.display(),
            check.contains,
            body
        );
    }

    drop(remote_fixture);
    drop(daemon);
}

/// Represents a running in-process daemon for one example execution.
struct RunningDaemon {
    socket: PathBuf,
    runtime: tokio::runtime::Runtime,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for RunningDaemon {
    /// Aborts the background server and synchronizes runtime shutdown.
    fn drop(&mut self) {
        self.server.abort();
        self.runtime.block_on(async {
            let _ = (&mut self.server).await;
        });
    }
}

const REMOTE_ENDPOINT_PLACEHOLDER: &str = "__TAK_REMOTE_ENDPOINT__";
const FIXTURE_TOR_ONION_HOST: &str =
    "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion";

/// Represents one running deterministic remote fixture for catalog execution.
struct RunningRemoteFixture {
    endpoint: String,
    env_overrides: Vec<(String, String)>,
    runtime: tokio::runtime::Runtime,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for RunningRemoteFixture {
    /// Aborts the background remote fixture server and synchronizes runtime shutdown.
    fn drop(&mut self) {
        self.server.abort();
        self.runtime.block_on(async {
            let _ = (&mut self.server).await;
        });
    }
}

/// Starts a deterministic remote fixture when the catalog case requires one.
fn maybe_start_remote_fixture(
    case: &ExampleCase,
    temp_root: &Path,
) -> Option<RunningRemoteFixture> {
    let fixture = case.remote_fixture?;
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for remote fixture");
    let listener = runtime
        .block_on(async { tokio::net::TcpListener::bind("127.0.0.1:0").await })
        .expect("bind local remote fixture listener");
    let port = listener
        .local_addr()
        .expect("remote fixture local addr")
        .port();

    let store_db = temp_root.join(format!("remote-v1-{}.sqlite", case.name.replace('/', "_")));
    let store = SubmitAttemptStore::with_db_path(store_db).expect("create submit attempt store");
    let server = runtime.spawn(async move {
        run_remote_v1_http_server(listener, store)
            .await
            .expect("remote fixture server should run");
    });
    wait_for_tcp_port(port, case);

    let (endpoint, env_overrides) = match fixture {
        RemoteFixtureKind::DirectHttp => (format!("http://127.0.0.1:{port}"), Vec::new()),
        RemoteFixtureKind::TorOnionHttp => (
            format!("http://{FIXTURE_TOR_ONION_HOST}:{port}"),
            vec![(
                "TAK_TEST_TOR_ONION_DIAL_ADDR".to_string(),
                format!("127.0.0.1:{port}"),
            )],
        ),
    };

    Some(RunningRemoteFixture {
        endpoint,
        env_overrides,
        runtime,
        server,
    })
}

/// Waits for one fixture TCP port to become available.
fn wait_for_tcp_port(port: u16, case: &ExampleCase) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "remote fixture did not become reachable for {} on 127.0.0.1:{}",
        case.name, port
    );
}

/// Rewrites placeholder endpoints inside copied TASKS files.
fn rewrite_remote_endpoint_placeholders(working: &Path, endpoint: &str) {
    rewrite_remote_endpoint_placeholders_in_dir(working, endpoint);
}

fn rewrite_remote_endpoint_placeholders_in_dir(root: &Path, endpoint: &str) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", root.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed reading entry in {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            rewrite_remote_endpoint_placeholders_in_dir(&path, endpoint);
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some("TASKS.py") {
            continue;
        }

        let original = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        if !original.contains(REMOTE_ENDPOINT_PLACEHOLDER) {
            continue;
        }

        let replaced = original.replace(REMOTE_ENDPOINT_PLACEHOLDER, endpoint);
        fs::write(&path, replaced)
            .unwrap_or_else(|err| panic!("failed writing {}: {err}", path.display()));
    }
}

/// Starts an in-process daemon when the example requires daemon-backed needs.
fn maybe_start_daemon(case: &ExampleCase, temp_root: &Path) -> Option<RunningDaemon> {
    if !case.requires_daemon {
        return None;
    }

    let socket = temp_root.join("takd.sock");
    let db_path = temp_root.join("takd.sqlite");

    let manager = new_shared_manager_with_db(db_path).expect("create sqlite-backed manager");
    {
        let mut guard = manager.lock().expect("lease manager lock");
        guard.set_capacity("cpu", Scope::Machine, None, 8.0);
        guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    }

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for daemon");
    let server_socket = socket.clone();
    let server_manager = Arc::clone(&manager);
    let server = runtime.spawn(async move {
        run_server(&server_socket, server_manager)
            .await
            .expect("daemon server should run");
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        socket.exists(),
        "daemon socket did not appear for example {}",
        case.name
    );

    Some(RunningDaemon {
        socket,
        runtime,
        server,
    })
}

/// Runs one `tak` command for an example and asserts expected outcome.
fn run_tak_command(
    working_dir: &Path,
    daemon_socket: Option<&Path>,
    args: &[&str],
    expect_success: bool,
    env_overrides: &[(String, String)],
) -> TakCommandOutput {
    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command.current_dir(working_dir).args(args);
    if let Some(socket) = daemon_socket {
        command.env("TAKD_SOCKET", socket);
    }
    for (key, value) in env_overrides {
        command.env(key, value);
    }

    let output = command.output().unwrap_or_else(|err| {
        panic!(
            "failed executing tak {:?} in {}: {err}",
            args,
            working_dir.display()
        )
    });
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let success = output.status.success();
    if success != expect_success {
        panic!(
            "unexpected status for args {:?} in {}: expected success={}, got success={}\nstdout:\n{}\nstderr:\n{}",
            args,
            working_dir.display(),
            expect_success,
            success,
            stdout,
            stderr
        );
    }

    TakCommandOutput { stdout, stderr }
}

/// Captured text output for one CLI invocation.
struct TakCommandOutput {
    stdout: String,
    stderr: String,
}

/// Validates run stdout/stderr substring expectations for one catalog case.
fn assert_expected_substrings(case: &ExampleCase, stdout: &str, stderr: &str) {
    for expected in &case.expect_stdout_contains {
        assert!(
            stdout.contains(expected),
            "stdout contract mismatch for {}: expected to contain `{}`\nstdout:\n{}\nstderr:\n{}",
            case.name,
            expected,
            stdout,
            stderr
        );
    }
    for expected in &case.expect_stderr_contains {
        assert!(
            stderr.contains(expected),
            "stderr contract mismatch for {}: expected to contain `{}`\nstdout:\n{}\nstderr:\n{}",
            case.name,
            expected,
            stdout,
            stderr
        );
    }
}

/// Validates expected output file presence for one catalog case.
fn assert_expected_output_files(case: &ExampleCase, working: &Path) {
    for relative in &case.check_files {
        let output = working.join(relative);
        assert!(
            output.exists(),
            "expected output file missing for {}: {}",
            case.name,
            output.display()
        );
    }
}

/// Recursively copies one directory tree into a destination path.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", dst.display()));

    let entries = fs::read_dir(src)
        .unwrap_or_else(|err| panic!("failed to read source dir {}: {err}", src.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed reading entry in {}: {err}", src.display()));
        let path = entry.path();
        let target = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap_or_else(|err| {
                panic!(
                    "failed to copy {} to {}: {err}",
                    path.display(),
                    target.display()
                )
            });
        }
    }
}
