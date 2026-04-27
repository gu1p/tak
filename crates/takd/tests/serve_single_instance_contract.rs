use crate::support;

use std::process::{Command as StdCommand, Stdio};

use support::cli::{roots, takd_bin};

#[test]
fn serve_rejects_second_process_for_the_same_state_root_before_transport_startup() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = roots(temp.path());

    let init = StdCommand::new(takd_bin())
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
            "--transport",
            "direct",
            "--base-url",
            "http://127.0.0.1:0",
            "--node-id",
            "single-instance",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut first = StdCommand::new(takd_bin())
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn first takd serve");

    let show = StdCommand::new(takd_bin())
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
    assert!(show.status.success(), "first serve should become ready");

    let second = StdCommand::new(takd_bin())
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run second takd serve");

    first.kill().expect("kill first takd serve");
    first.wait().expect("wait first takd serve");

    assert!(!second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("another takd serve process already owns state root"),
        "second serve should fail on the state-root lock before transport startup:\n{stderr}"
    );
}
