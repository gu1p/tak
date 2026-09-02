use crate::support;

use std::net::TcpListener;
use std::process::Stdio;

use support::daemon_command_paths::DaemonCommandPaths;

#[test]
fn status_reports_verified_reachability_after_tor_token_is_published() {
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
        .args(["--node-id", "builder-status"])
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
        .expect("run takd token show");
    assert!(show.status.success(), "takd token show should succeed");

    let status = paths
        .rooted_command(&support::takd_bin(), "status")
        .output()
        .expect("run takd status");

    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status.status.success(), "takd status should succeed");
    assert!(
        stdout.contains("transport: tor")
            && stdout.contains("readiness: advertised")
            && stdout.contains("transport_state: ready")
            && stdout.contains("reachability: verified")
            && stdout.contains("base_url: http://builder-status.onion")
            && !stdout.contains("http://[redacted].onion"),
        "missing ready status fields:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "log_path: {}",
            state_root
                .strip_prefix(temp.path())
                .expect("relative state root")
                .join("service.log")
                .display()
        )) && stdout.contains("log_state: present"),
        "missing log metadata:\n{stdout}"
    );
}
