use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn collect_tasks_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read shipped tasks directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_tasks_files(&path, files);
        } else if path.file_name().is_some_and(|name| name == "TASKS.py") {
            files.push(path);
        }
    }
}

#[test]
fn every_shipped_tasks_module_uses_the_v2_authoring_contract() {
    let root = repo_root();
    let mut files = vec![root.join("TASKS.py")];
    collect_tasks_files(&root.join("examples"), &mut files);
    collect_tasks_files(
        &root.join("docker/tor-test/remote_probe_project"),
        &mut files,
    );
    for path in files {
        let body = fs::read_to_string(&path).expect("read TASKS.py");
        let display = path.strip_prefix(&root).unwrap_or(&path).display();
        assert!(
            body.contains("spec_version=2"),
            "{display} is not explicit v2"
        );
        assert!(
            !body.contains("RemoteSelection.Shuffle"),
            "{display} uses removed Shuffle"
        );
    }
}

#[test]
fn state_and_environment_dependencies_are_explicit() {
    let root = repo_root();
    let root_tasks = fs::read_to_string(root.join("TASKS.py")).expect("root TASKS.py");
    for token in [
        "RemoteSelection.Balanced()",
        "session(\n    \"check-distributed\"",
        "session(\n    \"check-isolated\"",
        "SessionReuse.Paths(",
        "SessionReuse.Workspace()",
        "path(\".tmp/cargo-home\")",
        "path(\".tmp/cargo-target-local\")",
        "TAK_TEST_TMPDIR=\"/tmp/tak-tests-$TAK_RUN_ID-$TAK_JOB_ID\"",
        "CARGO_INCREMENTAL=0",
        "CARGO_PROFILE_DEV_DEBUG=0",
        "CARGO_PROFILE_TEST_DEBUG=0",
        "CARGO_BUILD_JOBS=2",
    ] {
        assert!(root_tasks.contains(token), "root TASKS.py missing {token}");
    }
    assert!(!root_tasks.contains("cascade_execution"));
    assert!(!root_tasks.contains("native-dead-code"));
    assert!(!root_tasks.contains("check-parallel"));
    assert!(!root_tasks.contains("cascade_session=True"));
    assert!(!root_tasks.contains("SessionReuse.Container()"));
    assert!(!root_tasks.contains("pass_env="));
    assert!(!root_tasks.contains("/var/tmp/tak-tests"));
    assert!(!root_tasks.contains("$PWD/.tmp/test-tmp"));
    assert!(!root_tasks.contains("${TAK_TEST_TMPDIR"));
    assert!(!root_tasks.contains("${CARGO_BUILD_JOBS"));

    let shared =
        fs::read_to_string(root.join("examples/large/31_remote_session_share_workspace/TASKS.py"))
            .expect("shared workspace TASKS.py");
    for token in [
        "SessionReuse.SharedWorkspace(max_parallel_tasks=2)",
        "Affinity.RequireSameNode(\"workspace-state\")",
        "outputs=[path(\"out/prepare-workspace.txt\")]",
    ] {
        assert!(
            shared.contains(token),
            "shared workspace example missing {token}"
        );
    }
}

#[test]
fn every_executable_example_declares_its_daemon_requirement() {
    let catalog =
        fs::read_to_string(repo_root().join("examples/catalog.toml")).expect("examples catalog");
    assert!(
        !catalog.contains("requires_daemon = false"),
        "all v2 execution goes through the local daemon"
    );
}
