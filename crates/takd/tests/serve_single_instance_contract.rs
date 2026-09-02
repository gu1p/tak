use crate::support;

use std::process::Stdio;

use support::cli::{roots, takd_bin};
use support::daemon_command_paths::DaemonCommandPaths;

#[test]
fn serve_rejects_second_process_for_the_same_state_root_before_transport_startup() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = roots(temp.path());
    let paths = DaemonCommandPaths::new(&config_root, &state_root);

    let init = paths
        .rooted_command(&takd_bin(), "init")
        .args([
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

    let mut first = paths
        .rooted_command(&takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn first takd serve");

    let show = paths
        .state_command(&takd_bin(), &["token", "show"])
        .args(["--wait", "--timeout-secs", "5"])
        .output()
        .expect("run token show");
    assert!(show.status.success(), "first serve should become ready");

    let second = paths
        .rooted_command(&takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
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
