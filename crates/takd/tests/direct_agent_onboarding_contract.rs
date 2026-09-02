use crate::support;

use std::fs;
use std::process::Stdio;

use support::{
    cli::{roots, takd_bin},
    daemon_command_paths::DaemonCommandPaths,
};

#[test]
fn direct_init_requires_base_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = roots(temp.path());
    let paths = DaemonCommandPaths::new(&config_root, &state_root);

    let init = paths
        .rooted_command(&takd_bin(), "init")
        .args(["--transport", "direct"])
        .output()
        .expect("run takd init");

    assert!(!init.status.success());
    assert!(String::from_utf8_lossy(&init.stderr).contains("base_url is required"));
}

#[test]
fn direct_serve_persists_token_for_remote_add_onboarding() {
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
            "--pool",
            "build",
            "--tag",
            "builder",
            "--capability",
            "linux",
            "--node-id",
            "builder-a",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = paths
        .rooted_command(&takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = paths
        .state_command(&takd_bin(), &["token", "show"])
        .args(["--wait", "--timeout-secs", "5"])
        .output()
        .expect("run token show");
    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(show.status.success(), "token show should succeed");
    let config = fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(config.contains("transport = \"direct\""));
    assert!(config.contains("build"), "missing build pool:\n{config}");
    assert!(
        !config.contains("base_url = \"http://127.0.0.1:0\""),
        "expected serve to persist a usable direct base_url:\n{config}"
    );
    let token = String::from_utf8_lossy(&show.stdout);
    assert!(token.trim().starts_with("takd:v2:"), "{token}");
    let payload = tak_proto::decode_remote_token(token.trim()).expect("decode direct v2 invite");
    assert_eq!(payload.version, "v2");
}
