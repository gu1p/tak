use crate::support;

use std::fs;
use std::net::TcpListener;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use support::daemon_command_paths::DaemonCommandPaths;

#[test]
fn serve_creates_service_log_with_tor_startup_milestones() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let paths = DaemonCommandPaths::new(&config_root, &state_root);
    let bind_addr = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").to_string()
    };

    let init = paths
        .rooted_command(&support::takd_bin(), "init")
        .args(["--node-id", "builder-logs"])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = paths
        .rooted_command(&support::takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
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

    let log_path = state_root.join("service.log");
    let deadline = Instant::now() + Duration::from_secs(5);
    let ready_line = "takd worker v2 onion service ready at http://builder-logs.onion";
    let contents = loop {
        if let Ok(contents) = fs::read_to_string(&log_path)
            && contents.contains("starting takd serve for transport tor")
            && contents.contains(ready_line)
        {
            break contents;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for service log"
        );
        thread::sleep(Duration::from_millis(50));
    };

    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(log_path.exists(), "service log should exist");
    assert!(
        contents.contains("starting takd serve for transport tor"),
        "missing start milestone:\n{contents}"
    );
    assert!(
        contents.contains(ready_line),
        "missing onion readiness milestone:\n{contents}"
    );
    assert!(
        !contents.contains("http://[redacted].onion"),
        "service log should not redact the full onion url:\n{contents}"
    );
}
