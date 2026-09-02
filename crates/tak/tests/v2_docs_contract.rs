use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn user_docs_explain_daemon_owned_runs_and_v2_migration() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("README");
    let guide = fs::read_to_string(root.join("docs/daemon-runs-v2.md")).expect("v2 guide");
    let docs = format!("{readme}\n{guide}");

    for token in [
        "tak runs list",
        "tak runs show",
        "tak runs attach",
        "tak runs cancel",
        "tak runs outputs",
        "disconnect does not cancel",
        "second Ctrl-C",
        "RemoteSelection.Balanced()",
        "RemoteSelection.RoundRobin()",
        "RemoteSelection.Sequential()",
        "SessionReuse.SharedWorkspace(max_parallel_tasks=N)",
        "Affinity.RequireSameNode",
        "pass_env",
        "--pass-env",
        "spec_version=2",
        "upgrade `tak` and `takd` together",
    ] {
        assert!(docs.contains(token), "v2 user docs missing `{token}`");
    }
}

#[test]
fn architecture_docs_assign_resolution_and_execution_ownership() {
    let root = repo_root();
    let architecture = fs::read_to_string(root.join("ARCHITECTURE.md")).expect("architecture");
    for token in [
        "Tak resolves Python policies",
        "concrete placement candidates",
        "takd owns scheduling",
        "retries, cancellation, artifacts, and events",
        "direct and Tor inventory",
    ] {
        assert!(
            architecture.contains(token),
            "architecture missing `{token}`"
        );
    }
}

#[test]
fn user_docs_do_not_teach_client_owned_or_v1_remote_access() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("README");

    for removed in ["/v1/", "tak remote add` still performs its own probe"] {
        assert!(
            !readme.contains(removed),
            "README still teaches removed remote access `{removed}`"
        );
    }
}

#[test]
fn shipped_tor_diagnostics_use_the_worker_v2_surface() {
    let root = repo_root();
    for relative in [
        "docker/tor-test/HTTP2_HANDOFF.md",
        "docker/tor-test/comm_test.sh",
        "docker/tor-test/diag_comm.sh",
        "docker/tor-test/proto_check.sh",
        "docker/tor-test/proto_diag.sh",
        "docker/tor-test/remote_run_probe.sh",
    ] {
        let body = fs::read_to_string(root.join(relative)).expect("shipped Tor diagnostic");
        assert!(
            !body.contains("/v1/"),
            "{relative} still targets the removed worker v1 surface"
        );
        assert!(
            body.contains("/v2/worker"),
            "{relative} does not identify the worker v2 surface"
        );
    }
}
