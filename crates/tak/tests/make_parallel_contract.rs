//! Black-box contract for parallel Make prerequisites and the parent join.

#![cfg(unix)]

use crate::support::make_runtime::start_local_daemon;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn parallel_make_prerequisites_overlap_and_parent_runs_after_join() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        r#".PHONY: bla lint test
# tak: parallel=lint,test
bla: lint test
	@test -f lint.done
	@test -f test.done
	@printf 'bla\n' >> invocations.log
lint:
	@printf 'lint\n' >> invocations.log
	@touch lint.started
	@for i in $$(seq 1 100); do test -f test.started && break; sleep 0.02; done
	@test -f test.started
	@touch lint.done
test:
	@printf 'test\n' >> invocations.log
	@touch test.started
	@for i in $$(seq 1 100); do test -f lint.started && break; sleep 0.02; done
	@test -f lint.started
	@touch test.done
"#,
    )?;

    let mut environment = BTreeMap::new();
    let _daemon = start_local_daemon(workspace.path(), &mut environment);
    let output = run_tak_output(workspace.path(), &["make", "bla"], &environment)?;

    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("invocations.log"))?
            .lines()
            .filter(|line| *line == "lint")
            .count(),
        1
    );
    assert!(
        workspace.path().join("bla").with_extension("done").exists() || {
            fs::read_to_string(workspace.path().join("invocations.log"))?.contains("bla")
        }
    );
    Ok(())
}
