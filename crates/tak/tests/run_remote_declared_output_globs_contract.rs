//! Black-box contract for explicit remote glob output syncing.

mod support;

use std::fs;

use anyhow::Result;

use support::remote_declared_outputs::{attach_direct_remote, remote_env};
use support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn run_remote_syncs_declared_output_globs_only() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = support::live_direct::LiveDirectRoots::new(temp.path());
    fs::create_dir_all(&workspace_root)?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task(
  "check",
  outputs=[glob("reports/**")],
  execution=RemoteOnly(Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps())),
  steps=[cmd("sh", "-ceu", """
mkdir -p reports/nested artifacts
printf 'keep-one\n' > reports/summary.txt
printf 'keep-two\n' > reports/nested/detail.txt
printf 'skip\n' > artifacts/detail.json
""")],
)])
SPEC
"#,
    )?;
    let _agent = attach_direct_remote(&workspace_root, &roots);

    let stdout = run_tak_expect_success(&workspace_root, &["run", "check"], &remote_env(&roots))?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace_root.join("reports/summary.txt"))?,
        "keep-one\n"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("reports/nested/detail.txt"))?,
        "keep-two\n"
    );
    assert!(
        !workspace_root.join("artifacts/detail.json").exists(),
        "non-matching glob output should not be synced"
    );
    Ok(())
}

#[test]
fn run_remote_fails_when_declared_output_is_missing() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = support::live_direct::LiveDirectRoots::new(temp.path());
    fs::create_dir_all(&workspace_root)?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task(
  "check",
  outputs=[path("out/missing.txt")],
  execution=RemoteOnly(Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps())),
  steps=[cmd("sh", "-c", "printf 'done\n'")],
)])
SPEC
"#,
    )?;
    let _agent = attach_direct_remote(&workspace_root, &roots);

    let (_stdout, stderr) =
        run_tak_expect_failure(&workspace_root, &["run", "check"], &remote_env(&roots))?;

    assert!(
        stderr.contains("declared output"),
        "stderr should mention the missing declared output:\n{stderr}"
    );
    Ok(())
}
