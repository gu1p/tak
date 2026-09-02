use std::fs;

use anyhow::Result;

use crate::support::exec_daemon::ExecDaemon;
use crate::support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn daemon_run_materializes_only_declared_output_globs() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, SUCCESS_TASKS)?;
    let daemon = ExecDaemon::spawn(temp.path(), &workspace);

    let stdout = run_tak_expect_success(&workspace, &["run", "//:check"], daemon.environment())?;

    assert!(stdout.contains("output_committing"), "{stdout}");
    assert_eq!(fs::read(workspace.join("reports/summary.txt"))?, b"one\n");
    assert_eq!(fs::read(workspace.join("reports/nested.txt"))?, b"two\n");
    assert!(!workspace.join("scratch/leak.txt").exists());
    Ok(())
}

#[test]
fn daemon_run_reports_missing_declared_output_after_task_stderr() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, MISSING_TASKS)?;
    let daemon = ExecDaemon::spawn(temp.path(), &workspace);

    let (stdout, stderr) =
        run_tak_expect_failure(&workspace, &["run", "//:check"], daemon.environment())?;
    assert!(stderr.contains("diagnostic-line"), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("declared output") || stderr.contains("declared output"),
        "{stdout}\n{stderr}"
    );
    Ok(())
}

const SUCCESS_TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "check", outputs=[glob("reports/**")], steps=[cmd("sh", "-c",
  "mkdir -p reports scratch && echo one > reports/summary.txt && echo two > reports/nested.txt && echo leak > scratch/leak.txt")],
)])
SPEC
"#;

const MISSING_TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "check", outputs=[path("reports/missing.txt")],
  steps=[cmd("sh", "-c", "echo diagnostic-line >&2")],
)])
SPEC
"#;
