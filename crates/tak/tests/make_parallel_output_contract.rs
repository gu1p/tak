//! Black-box contract for configurable parallel Make output.

#![cfg(unix)]

use crate::support::make_runtime::start_local_daemon;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn parallel_make_output_is_live_by_default_and_cli_can_group_it() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: all left right
# tak: parallel=left,right
all: left right
	@printf 'joined\n'
left:
	@printf 'left-one\nleft-two\n'
right:
	@printf 'right-one\nright-two\n'
"#,
    )?;

    let mut live_environment = BTreeMap::new();
    let _live_daemon = start_local_daemon(workspace.path(), &mut live_environment);
    let live = run_tak_output(workspace.path(), &["make", "all"], &live_environment)?;
    assert!(live.status.success());
    let live_stdout = String::from_utf8_lossy(&live.stdout);
    assert!(live_stdout.contains("[left] left-one"), "{live_stdout}");
    assert!(live_stdout.contains("[right] right-one"), "{live_stdout}");

    drop(_live_daemon);
    let mut grouped_environment = BTreeMap::new();
    let _grouped_daemon = start_local_daemon(workspace.path(), &mut grouped_environment);
    let grouped = run_tak_output(
        workspace.path(),
        &["make", "--parallel-output", "grouped", "all"],
        &grouped_environment,
    )?;
    assert!(grouped.status.success());
    let grouped_stdout = String::from_utf8_lossy(&grouped.stdout);
    assert!(grouped_stdout.contains("[left] left-one\n[left] left-two"));
    assert!(grouped_stdout.contains("[right] right-one\n[right] right-two"));
    Ok(())
}
