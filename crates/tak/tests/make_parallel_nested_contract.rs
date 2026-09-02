//! Black-box contract for recursive parallel Make groups.

#![cfg(unix)]

use crate::support::make_runtime::start_local_daemon;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn nested_parallel_groups_form_a_recursive_join_graph() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: all checks build lint test
# tak: parallel=checks,build
all: checks build
	@test -f checks.done
	@test -f build.done
	@touch all.done
# tak: parallel=lint,test
checks: lint test
	@test -f lint.done
	@test -f test.done
	@touch checks.done
lint:
	@touch lint.started
	@for i in $$(seq 1 500); do test -f test.started -a -f build.started && break; sleep 0.02; done
	@test -f test.started -a -f build.started
	@touch lint.done
test:
	@touch test.started
	@for i in $$(seq 1 500); do test -f lint.started -a -f build.started && break; sleep 0.02; done
	@test -f lint.started -a -f build.started
	@touch test.done
build:
	@touch build.started
	@for i in $$(seq 1 500); do test -f lint.started -a -f test.started && break; sleep 0.02; done
	@test -f lint.started -a -f test.started
	@touch build.done
"#,
    )?;

    let mut environment = BTreeMap::new();
    let _daemon = start_local_daemon(workspace.path(), &mut environment);
    let output = run_tak_output(workspace.path(), &["make", "all"], &environment)?;

    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(workspace.path().join("all.done").exists());
    Ok(())
}
