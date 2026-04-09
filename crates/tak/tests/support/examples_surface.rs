#![allow(dead_code)]

use std::collections::BTreeMap;

use anyhow::Result;

use super::examples_catalog::{ExampleEntry, repo_root};
use super::run_tak_expect_success;

pub fn verify_cli_surface(entry: &ExampleEntry) -> Result<()> {
    let env = BTreeMap::new();
    let example_dir = repo_root().join("examples").join(&entry.name);

    let list = run_tak_expect_success(&example_dir, &["list"], &env)?;
    assert!(list.contains(&entry.explain_target), "list output:\n{list}");

    let explain = run_tak_expect_success(&example_dir, &["explain", &entry.explain_target], &env)?;
    let expected = format!("label: {}", entry.explain_target);
    assert!(explain.contains(&expected), "explain output:\n{explain}");

    let graph = run_tak_expect_success(
        &example_dir,
        &["graph", &entry.explain_target, "--format", "dot"],
        &env,
    )?;
    assert!(graph.contains("digraph tak"), "graph output:\n{graph}");

    Ok(())
}
