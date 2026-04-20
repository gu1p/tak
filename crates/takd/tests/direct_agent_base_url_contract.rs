use crate::support;

use std::process::Command as StdCommand;

use support::cli::{roots, takd_bin};

#[test]
fn direct_init_rejects_base_url_with_unsupported_components() {
    for base_url in [
        "http://user:pass@127.0.0.1:0",
        "http://127.0.0.1:0/prefix",
        "http://127.0.0.1:0?query=1",
        "http://127.0.0.1:0#fragment",
    ] {
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
                base_url,
            ])
            .output()
            .expect("run takd init");

        assert!(!init.status.success(), "takd init should reject {base_url}");
        assert!(
            String::from_utf8_lossy(&init.stderr)
                .contains("base_url must not include userinfo, path, query, or fragment"),
            "unexpected stderr for {base_url}:\n{}",
            String::from_utf8_lossy(&init.stderr)
        );
    }
}

#[test]
fn direct_init_rejects_base_url_without_explicit_port() {
    for base_url in [
        "http://127.0.0.1",
        "https://builder.example",
        "http://[::1]",
    ] {
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
                base_url,
            ])
            .output()
            .expect("run takd init");

        assert!(!init.status.success(), "takd init should reject {base_url}");
        assert!(
            String::from_utf8_lossy(&init.stderr).contains("base_url must include a port"),
            "unexpected stderr for {base_url}:\n{}",
            String::from_utf8_lossy(&init.stderr)
        );
    }
}
