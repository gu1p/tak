#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use super::examples_catalog::ExampleEntry;
use super::run_timeout::{
    run_tak_expect_failure_with_timeout, run_tak_expect_success_with_timeout,
};

const EXAMPLE_CLIENT_TIMEOUT: Duration = Duration::from_secs(180);

pub fn assert_example_run(
    entry: &ExampleEntry,
    workspace_root: &Path,
    env: &BTreeMap<String, String>,
) -> Result<()> {
    if entry.expect_success {
        let stdout = run_tak_expect_success_with_timeout(
            workspace_root,
            &["run", &entry.run_target],
            env,
            EXAMPLE_CLIENT_TIMEOUT,
        )?;
        return assert_needles("stdout", &stdout, &entry.expect_stdout_contains);
    }

    let (stdout, stderr) = run_tak_expect_failure_with_timeout(
        workspace_root,
        &["run", &entry.run_target],
        env,
        EXAMPLE_CLIENT_TIMEOUT,
    )?;
    assert_needles("stdout", &stdout, &entry.expect_stdout_contains)?;
    assert_needles("stderr", &stderr, &entry.expect_stderr_contains)
}

pub fn assert_example_outputs(entry: &ExampleEntry, workspace_root: &Path) -> Result<()> {
    for relative in &entry.check_files {
        let path = workspace_root.join(relative);
        assert!(path.is_file(), "missing expected output {}", path.display());
    }
    for check in &entry.check_file_contains {
        let body = fs::read_to_string(workspace_root.join(&check.path))
            .with_context(|| format!("failed to read {} for {}", check.path, entry.name))?;
        assert!(
            body.contains(&check.contains),
            "file {} missing `{}`",
            check.path,
            check.contains
        );
    }
    Ok(())
}

fn assert_needles(kind: &str, haystack: &str, needles: &[String]) -> Result<()> {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "{kind} missing `{needle}`\n{haystack}"
        );
    }
    Ok(())
}
