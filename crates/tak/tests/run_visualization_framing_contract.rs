//! Stream-framing contracts for prefixed `tak run` output.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::run_tak_output;

#[test]
fn parallel_fragments_are_buffered_per_task_and_trailing_bytes_are_flushed() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("alpha", steps=[cmd("sh", "-c", "printf 'alpha-'; sleep 0.05; printf 'line\\n'")]),
  task("beta", steps=[cmd("sh", "-c", "printf 'beta-'; sleep 0.02; printf 'line\\n'")]),
  task("tail", steps=[cmd("sh", "-c", "printf 'tail-fragment'")]),
])
SPEC
"#,
    )?;

    let output = run_tak_output(
        temp.path(),
        &["run", "--jobs", "3", "//:alpha", "//:beta", "//:tail"],
        &BTreeMap::new(),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("[//:alpha@local] alpha-line\n"), "{stdout}");
    assert!(stdout.contains("[//:beta@local] beta-line\n"), "{stdout}");
    assert!(
        stdout.contains("[//:tail@local] tail-fragment\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("[//:alpha@local] alpha-\n"), "{stdout}");
    assert!(!stdout.contains("[//:beta@local] beta-\n"), "{stdout}");
    Ok(())
}
