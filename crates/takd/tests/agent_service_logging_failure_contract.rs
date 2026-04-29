use crate::support;

use std::fs;
use std::net::TcpListener;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn logs_include_retryable_tor_startup_failure_details() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let bind_addr = listener.local_addr().expect("addr").to_string();
    drop(listener);

    let init = StdCommand::new(support::takd_bin())
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
            "--node-id",
            "builder-log-failure",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = StdCommand::new(support::takd_bin())
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
        .env("TAKD_TEST_TOR_FAIL_STARTUP_ONCE", "1")
        .env("TAKD_TOR_RECOVERY_INITIAL_BACKOFF_MS", "50")
        .env("TAKD_TOR_RECOVERY_MAX_BACKOFF_MS", "50")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "5",
        ])
        .output()
        .expect("run token show");
    assert!(show.status.success(), "token show should succeed");

    let expected = "test startup failure hook triggered";
    wait_for_log(&state_root, expected);
    let logs = StdCommand::new(support::takd_bin())
        .args(["logs", "--state-root", &state_root.display().to_string()])
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
