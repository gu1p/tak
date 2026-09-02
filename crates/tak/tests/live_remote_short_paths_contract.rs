use crate::support;

use anyhow::Result;

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn live_remote_harnesses_run_daemons_with_short_relative_paths() -> Result<()> {
    for relative in [
        "crates/tak/tests/support/live_direct.rs",
        "crates/tak/tests/support/live_tor.rs",
    ] {
        let source = std::fs::read_to_string(repo_root().join(relative))?;
        assert!(
            source.contains("DaemonCommandPaths"),
            "{relative} must derive short daemon command paths"
        );
        assert!(
            source.contains("paths.rooted_command(takd"),
            "{relative} must resolve relative daemon paths from their shared temp root"
        );
        assert!(
            source.contains("paths.remote_exec_root()"),
            "{relative} must resolve the remote execution root relative to the temp root"
        );
        assert!(
            source.contains(".env(\"XDG_RUNTIME_DIR\", paths.runtime_root())"),
            "{relative} must isolate the spawned daemon runtime socket"
        );
        assert!(
            source.contains(".env(\"XDG_CONFIG_HOME\", paths.config_root())"),
            "{relative} must isolate the spawned daemon remote inventory"
        );
    }
    let tor = std::fs::read_to_string(repo_root().join("crates/tak/tests/support/live_tor.rs"))?;
    assert!(tor.contains("state_command(takd"));
    Ok(())
}

#[test]
fn short_paths_keep_control_socket_below_unix_limit() {
    let root = std::path::Path::new("/checkout/with/a/very/long/path/used/by/the/test/harness");
    let config = root.join("server-config");
    let state = root.join("server-state");
    let paths = support::short_daemon_paths::DaemonCommandPaths::new(&config, &state);

    assert_eq!(paths.command_root(), root);
    assert_eq!(paths.config_root(), std::path::Path::new("server-config"));
    assert_eq!(paths.state_root(), std::path::Path::new("server-state"));
    assert_eq!(
        paths.remote_exec_root(),
        std::path::Path::new("remote-exec")
    );
    assert_eq!(paths.runtime_root(), std::path::Path::new("runtime"));
    assert!(
        paths
            .state_root()
            .join("agent-control.sock")
            .as_os_str()
            .len()
            < 104
    );

    let command = paths.rooted_command(std::path::Path::new("takd"), "serve");
    assert_eq!(command.get_current_dir(), Some(root));
}

#[test]
fn direct_roots_remain_absolute_for_logs_and_assertions() {
    let base = std::env::current_dir()
        .expect("current directory")
        .join(".tmp/live-direct-contract");
    let roots = support::live_direct::LiveDirectRoots::new(&base);

    assert_eq!(roots.server_config_root, base.join("server-config"));
    assert_eq!(roots.server_state_root, base.join("server-state"));
}
