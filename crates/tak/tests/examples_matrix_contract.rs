#![recursion_limit = "256"]

#[cfg(unix)]
mod process_timeout_contract;
mod support;

use anyhow::Result;

use support::examples_catalog::{assert_no_failures, load_catalog, repo_root};
use support::examples_run::run_example;
use support::examples_surface::verify_cli_surface;

#[test]
fn full_matrix_process_cap_uses_a_fixture_specific_match() -> Result<()> {
    let tasks = std::fs::read_to_string(
        repo_root().join("examples/large/24_full_feature_matrix_end_to_end/TASKS.py"),
    )?;

    assert!(
        tasks.contains("match=\"tak-example-large-24-simulator\""),
        "the full-matrix process cap must not match common coverage build processes such as \
         simd-adler32, similar, or strsim"
    );
    Ok(())
}

#[test]
fn catalog_examples_support_list_explain_and_graph_from_real_directories() -> Result<()> {
    let catalog = load_catalog()?;
    let mut failures = Vec::new();

    for entry in &catalog.example {
        if let Err(err) = verify_cli_surface(entry) {
            failures.push(format!("{}: {err:#}", entry.name));
        }
    }

    assert_no_failures("real example directory surface checks", failures);
    Ok(())
}

#[test]
fn catalog_examples_run_targets_from_staged_workspaces() -> Result<()> {
    let catalog = load_catalog()?;
    let mut failures = Vec::new();

    for entry in &catalog.example {
        let temp = tempfile::tempdir()?;
        if let Err(err) = run_example(entry, temp.path()) {
            failures.push(format!("{}: {err:#}", entry.name));
        }
    }

    assert_no_failures("staged example run checks", failures);
    Ok(())
}
