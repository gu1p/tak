use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::make_runtime::start_local_daemon;
use crate::support::terminal::run_tak_terminal;

#[test]
fn dashboard_capture_preserves_the_original_parallel_make_failure_code() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: all first survivor
# tak: parallel=first,survivor
all: first survivor
	@exit 99
first:
	@exit 23
survivor:
	@printf 'survivor-finished\n'
"#,
    )?;

    let mut environment = BTreeMap::new();
    let _daemon = start_local_daemon(workspace.path(), &mut environment);
    let output = run_tak_terminal(workspace.path(), &["make", "all"], &environment)?;
    let terminal = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(23), "{terminal:?}");
    assert!(terminal.contains("TAK"), "{terminal:?}");
    assert!(terminal.contains("survivor-finished"), "{terminal:?}");
    assert!(terminal.contains("\u{1b}[?1049l"), "{terminal:?}");
    Ok(())
}
