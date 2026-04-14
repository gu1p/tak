use std::net::TcpListener;
use std::process::{Command as StdCommand, Stdio};

#[test]
fn status_reports_verified_reachability_after_tor_token_is_published() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let bind_addr = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").to_string()
    };

    let init = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
            "--node-id",
            "builder-status",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
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
        .expect("run takd token show");
    assert!(show.status.success(), "takd token show should succeed");

    let status = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "status",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
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
            && stdout.contains("base_url: http://builder-status.onion"),
        "missing ready status fields:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "log_path: {}",
            state_root.join("service.log").display()
        )) && stdout.contains("log_state: present"),
        "missing log metadata:\n{stdout}"
    );
}
