//! Contract for the repo test Dockerfile used by root TASKS.py.

use std::fs;
use std::path::Path;

use anyhow::Result;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn repo_test_dockerfile_trusts_mounted_git_worktrees() -> Result<()> {
    let dockerfile = fs::read_to_string(repo_root().join("docker/tak-tests/Dockerfile"))?;

    assert!(
        dockerfile.contains("git config --system --add safe.directory '*'"),
        "docker/tak-tests/Dockerfile must trust mounted git worktrees:\n{dockerfile}"
    );

    Ok(())
}

#[test]
fn repo_test_dockerfile_extends_live_tor_timeout_budget() -> Result<()> {
    let dockerfile = fs::read_to_string(repo_root().join("docker/tak-tests/Dockerfile"))?;

    assert!(
        dockerfile.contains("ENV TAK_LIVE_TOR_TEST_TIMEOUT_SECS=420"),
        "docker/tak-tests/Dockerfile must extend live Tor timeout budget:\n{dockerfile}"
    );

    Ok(())
}
