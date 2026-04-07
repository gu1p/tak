mod support;

use std::fs;
use std::process::{Command as StdCommand, Stdio};

use support::cli::{roots, takd_bin};

#[test]
fn direct_init_requires_base_url() {
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
        ])
        .output()
        .expect("run takd init");

    assert!(!init.status.success());
    assert!(String::from_utf8_lossy(&init.stderr).contains("base_url is required"));
}

#[test]
fn direct_serve_persists_token_for_remote_add_onboarding() {
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

    let mut child = StdCommand::new(takd_bin())
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
        .expect("spawn takd serve");

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
    assert!(
        String::from_utf8_lossy(&show.stdout)
            .trim()
            .starts_with("takd:v1:")
    );
}
