//! Contract for workspace test wiring and repo root tasks.

use std::fs;
use std::path::Path;

use anyhow::Result;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

fn read_repo_file(path: &str) -> Result<String> {
    Ok(fs::read_to_string(repo_root().join(path))?)
}

#[test]
fn workspace_lib_crates_do_not_bridge_suite_tests_through_prod_libs() -> Result<()> {
    for lib_rs in [
        "crates/tak-core/src/lib.rs",
        "crates/tak-loader/src/lib.rs",
        "crates/tak-proto/src/lib.rs",
        "crates/tak-runner/src/lib.rs",
        "crates/tak-exec/src/lib.rs",
        "crates/takd/src/lib.rs",
        "crates/tak/src/lib.rs",
    ] {
        let contents = read_repo_file(lib_rs)?;
        assert!(!contents.contains("mod external_tests;"));
    }

    for path in [
        "crates/tak-core/src/external_tests.rs",
        "crates/tak-loader/src/external_tests.rs",
        "crates/tak-proto/src/external_tests.rs",
        "crates/tak-runner/src/external_tests.rs",
        "crates/tak-exec/src/external_tests.rs",
        "crates/takd/src/external_tests.rs",
        "crates/tak/src/external_tests.rs",
    ] {
        assert!(!repo_root().join(path).exists());
    }
    Ok(())
}

#[test]
fn repo_root_tasks_py_does_not_force_local_container_runtime() -> Result<()> {
    let tasks = read_repo_file("TASKS.py")?;

    assert!(!tasks.contains("\"container_runtime\": DockerfileRuntime"));
    assert!(!tasks.contains("scripts/run_workspace_tests.sh"));
    assert!(!tasks.contains("scripts/run_check_rust.sh"));
    Ok(())
}
