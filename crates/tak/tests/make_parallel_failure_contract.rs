//! Black-box contract for deterministic parallel Make failures.

#![cfg(unix)]

use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn parallel_make_returns_first_listed_failure_after_siblings_finish() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: all slow fast survivor
# tak: parallel=slow,fast,survivor
all: slow fast survivor
	@touch parent-ran
slow:
	@sleep 0.15
	@exit 23
fast:
	@exit 42
survivor:
	@sleep 0.20
	@touch survivor.done
"#,
    )?;

    let output = run_tak_output(workspace.path(), &["make", "all"], &BTreeMap::new())?;

    assert_eq!(output.status.code(), Some(23));
    assert!(workspace.path().join("survivor.done").exists());
    assert!(!workspace.path().join("parent-ran").exists());
    Ok(())
}
