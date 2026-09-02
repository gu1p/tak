use crate::support;

use std::fs;
use std::net::TcpListener;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use support::daemon_command_paths::DaemonCommandPaths;

#[test]
fn logs_include_retryable_tor_startup_failure_details() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let paths = DaemonCommandPaths::new(&config_root, &state_root);
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let bind_addr = listener.local_addr().expect("addr").to_string();
    drop(listener);

    let init = paths
        .rooted_command(&support::takd_bin(), "init")
        .args(["--node-id", "builder-log-failure"])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = paths
        .rooted_command(&support::takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
        .env("TAKD_TEST_TOR_FAIL_STARTUP_ONCE", "1")
        .env("TAKD_TOR_RECOVERY_INITIAL_BACKOFF_MS", "50")
        .env("TAKD_TOR_RECOVERY_MAX_BACKOFF_MS", "50")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = paths
        .state_command(&support::takd_bin(), &["token", "show"])
        .args(["--wait", "--timeout-secs", "30"])
        .output()
        .expect("run token show");
    assert!(show.status.success(), "token show should succeed");

    let expected = "test startup failure hook triggered";
    wait_for_log(&state_root, expected);
    let logs = paths
        .state_command(&support::takd_bin(), &["logs"])
        .output()
        .expect("run takd logs");
    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(logs.status.success(), "takd logs should succeed");
    let stdout = String::from_utf8_lossy(&logs.stdout);
    assert!(
        stdout.contains(expected),
        "missing failure detail:\n{stdout}"
    );
}

fn wait_for_log(state_root: &std::path::Path, expected: &str) {
    let log_path = state_root.join("service.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(contents) = fs::read_to_string(&log_path)
            && contents.contains(expected)
        {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for service log detail `{expected}`");
}
