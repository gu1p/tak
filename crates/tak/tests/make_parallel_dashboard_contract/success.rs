use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::make_runtime::start_local_daemon;
use crate::support::terminal::run_tak_terminal;

#[test]
fn parallel_make_uses_the_daemon_owned_dashboard_and_restores_the_terminal() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: all left right
# tak: parallel=left,right
all: left right
	@printf 'joined\n'
left:
	@printf 'left-output\n'
right:
	@printf 'right-output\n'
"#,
    )?;

    let mut environment = BTreeMap::new();
    let _daemon = start_local_daemon(workspace.path(), &mut environment);
    let output = run_tak_terminal(workspace.path(), &["make", "all"], &environment)?;
    let terminal = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{terminal:?}");
    for text in [
        "TAK",
        "RUN",
        "NODES",
        "TASKS",
        "SCHEDULER",
        "QUEUE",
        "LIVE",
        "LOGS",
    ] {
        assert!(terminal.contains(text), "missing {text:?} in {terminal:?}");
    }
    assert!(terminal.contains("//:make-0"), "{terminal:?}");
    assert!(terminal.contains("\u{1b}[?1049h"), "screen not entered");
    assert!(terminal.contains("\u{1b}[?1049l"), "screen not restored");
    Ok(())
}
