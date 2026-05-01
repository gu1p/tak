use std::{collections::BTreeMap, fs};

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn first_available_plain_local_fallback_runs_in_repo_checkout_without_session() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("empty-config");
    fs::create_dir_all(&config)?;
    write_tasks(
        &workspace,
        r#"CONTAINER = Container.Image("alpine:3.20")
SESSION = session("remote-state", reuse=SessionReuse.Workspace())
EXEC = Execution.FirstAvailable([
  Execution.Remote(pool="missing", container=CONTAINER, session=SESSION),
  Execution.Local(),
])

SPEC = module_spec(tasks=[
  task("build", steps=[cmd("sh", "-c", "echo repo > state.txt")]),
  task("check", deps=[":build"], outputs=[path("out")], steps=[
    cmd("sh", "-c", "test -f state.txt && mkdir -p out && cat state.txt > out/result.txt")
  ], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".to_string(), config.display().to_string());
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert!(stdout.contains("session=none"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/result.txt"))?.trim(),
        "repo"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("state.txt"))?.trim(),
        "repo"
    );
    Ok(())
}

#[test]
fn local_execution_session_cascades_managed_workspace_to_dependencies() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SESSION = session("local-state", reuse=SessionReuse.Workspace())
EXEC = Execution.Local(session=SESSION)

SPEC = module_spec(tasks=[
  task("build", steps=[cmd("sh", "-c", "echo cached > state.txt")]),
  task("test", deps=[":build"], steps=[cmd("test", "-f", "state.txt")]),
  task("check", deps=[":test"], outputs=[path("out")], steps=[
    cmd("sh", "-c", "mkdir -p out && cat state.txt > out/result.txt")
  ], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &BTreeMap::new())?;

    assert!(stdout.contains("session=local-state"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/result.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join("state.txt").exists());
    Ok(())
}
