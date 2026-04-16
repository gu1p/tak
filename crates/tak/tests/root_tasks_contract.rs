//! Black-box contract for the repo root TASKS.py surface.

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use support::run_tak_expect_success;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn repo_root_tasks_surface_lists_tak_owned_workflow_targets() -> Result<()> {
    let env = BTreeMap::new();
    let list = run_tak_expect_success(repo_root(), &["list"], &env)?;

    for label in [
        "//:check",
        "//:coverage",
        "//:fmt-check",
        "//:line-limits-check",
        "//:src-test-separation-check",
        "//:workflow-contract-check",
        "//:generated-artifact-ignore-check",
        "//:lint",
        "//:test",
        "//:docs-check",
        "//:build-release-x86_64-unknown-linux-musl",
        "//:build-release-aarch64-unknown-linux-musl",
        "//:build-release-x86_64-apple-darwin",
        "//:build-release-aarch64-apple-darwin",
        "//:package-release-x86_64-unknown-linux-musl",
        "//:package-release-aarch64-unknown-linux-musl",
        "//:package-release-x86_64-apple-darwin",
        "//:package-release-aarch64-apple-darwin",
    ] {
        assert!(
            list.contains(label),
            "missing {label} in list output:\n{list}"
        );
    }

    let explain = run_tak_expect_success(repo_root(), &["explain", "//:check"], &env)?;
    assert!(explain.contains("label: //:check"), "explain:\n{explain}");

    let explain = run_tak_expect_success(
        repo_root(),
        &["explain", "//:package-release-x86_64-unknown-linux-musl"],
        &env,
    )?;
    assert!(
        explain.contains("label: //:package-release-x86_64-unknown-linux-musl"),
        "explain:\n{explain}"
    );

    Ok(())
}
