use crate::support;

use std::process::Command as StdCommand;

use support::cli::{roots, takd_bin};

#[test]
fn direct_init_normalizes_mixed_case_http_scheme() {
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
            "HTTP://127.0.0.1:43123",
        ])
        .output()
        .expect("run takd init");

    assert!(
        init.status.success(),
        "takd init should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let config = std::fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(
        config.contains("base_url = \"http://127.0.0.1:43123\""),
        "expected canonical lowercase base_url:\n{config}"
    );
    assert!(
        !config.contains("base_url = \"HTTP://127.0.0.1:43123\""),
        "did not expect mixed-case base_url to persist:\n{config}"
    );
}
