//! Black-box contract for explicit remote path output syncing.

use crate::support;

use std::fs;

use anyhow::Result;

use support::remote_declared_outputs::{attach_direct_remote, remote_env};
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_remote_syncs_only_declared_output_paths() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = support::live_direct::LiveDirectRoots::new(temp.path());
    fs::create_dir_all(&workspace_root)?;
    fs::write(workspace_root.join(".gitignore"), "target/\n")?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task(
  "check",
  context=CurrentState(ignored=[gitignore()]),
  outputs=[path("out")],
  execution=RemoteOnly(Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps(), runtime=ContainerRuntime(image="alpine:3.20"))),
  steps=[cmd("sh", "-ceu", """
mkdir -p out target/private
printf 'synced\n' > out/result.txt
printf 'ignored\n' > target/private/ghost.txt
""")],
)])
SPEC
"#,
    )?;
    let _agent = attach_direct_remote(&workspace_root, &roots);

    let stdout = run_tak_expect_success(&workspace_root, &["run", "check"], &remote_env(&roots))?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/result.txt"))?,
        "synced\n"
    );
    assert!(
        !workspace_root.join("target/private/ghost.txt").exists(),
        "undeclared output should not be synced"
    );
    Ok(())
}

#[test]
fn run_remote_without_declared_outputs_does_not_sync_created_files() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = support::live_direct::LiveDirectRoots::new(temp.path());
    fs::create_dir_all(&workspace_root)?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task(
  "check",
  execution=RemoteOnly(Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps(), runtime=ContainerRuntime(image="alpine:3.20"))),
  steps=[cmd("sh", "-ceu", """
mkdir -p out
printf 'remote only\n' > out/undeclared.txt
printf 'ran\n'
""")],
)])
SPEC
"#,
    )?;
    let _agent = attach_direct_remote(&workspace_root, &roots);

    let stdout = run_tak_expect_success(&workspace_root, &["run", "check"], &remote_env(&roots))?;

    assert!(stdout.contains("ran\n"), "stdout:\n{stdout}");
    assert!(
        !workspace_root.join("out/undeclared.txt").exists(),
        "undeclared outputs should remain remote-only"
    );
    Ok(())
}
