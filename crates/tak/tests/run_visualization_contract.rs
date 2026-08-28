//! Black-box contracts for the append-only `tak run` visualization.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::run_tak_output;

#[test]
fn redirected_run_prefixes_logs_and_renders_lifecycle_without_ansi() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("alpha", steps=[cmd("sh", "-c", "printf 'alpha-out\n'; printf 'alpha-err\n' >&2")]),
  task("beta", steps=[cmd("sh", "-c", "printf 'beta-out\n'")]),
  task("all", deps=[":alpha", ":beta"]),
])
SPEC
"#,
    )?;

    let output = run_tak_output(
        temp.path(),
        &["run", "--jobs", "2", "//:all"],
        &BTreeMap::new(),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("[//:alpha@local] alpha-out\n"), "{stdout}");
    assert!(stdout.contains("[//:beta@local] beta-out\n"), "{stdout}");
    assert!(stderr.contains("[//:alpha@local] alpha-err\n"), "{stderr}");
    assert!(
        stderr.contains("[waiting] //:alpha@pending scheduler #1"),
        "{stderr}"
    );
    assert!(stderr.contains("2 passed"), "{stderr}");
    assert!(!stderr.contains("[waiting] //:all@"), "{stderr}");
    assert!(
        !stderr.contains('\u{1b}'),
        "redirected stderr contained ANSI: {stderr:?}"
    );
    assert!(stdout.contains("//:alpha: ok (task_run_id="), "{stdout}");
    Ok(())
}

#[test]
fn one_task_still_uses_the_visual_prefix() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[task('solo', steps=[cmd('sh', '-c', 'echo solo')])])\nSPEC\n",
    )?;

    let output = run_tak_output(temp.path(), &["run", "//:solo"], &BTreeMap::new())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("[//:solo@local] solo\n"), "{stdout}");
    Ok(())
}

#[test]
fn planning_errors_finish_with_an_explicit_failed_frame() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("TASKS.py"),
        "SPEC = module_spec(tasks=[task('present', steps=[cmd('true')])])\nSPEC\n",
    )?;

    let output = run_tak_output(temp.path(), &["run", "//:missing"], &BTreeMap::new())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("tak run · failed"), "{stderr}");
    assert!(stderr.contains("missing"), "{stderr}");
    Ok(())
}
