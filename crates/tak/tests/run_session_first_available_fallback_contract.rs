use std::{collections::BTreeMap, fs};

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn first_available_session_local_fallback_preserves_cascaded_workspace() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("empty-config");
    fs::create_dir_all(&config)?;
    write_tasks(
        &workspace,
        r#"CONTAINER = Container.Image("alpine:3.20")
SESSION = session(
  "remote-or-local",
  execution=Execution.FirstAvailable([
    Execution.Remote(pool="missing", container=CONTAINER),
    Execution.Local(container=CONTAINER),
  ]),
  reuse=SessionReuse.Workspace(),
)

SPEC = module_spec(tasks=[
  task("build", steps=[cmd("sh", "-c", "mkdir -p .session && echo cached > .session/state.txt")]),
  task("check", deps=[":build"], outputs=[path("out")], steps=[
    cmd("sh", "-c", "test -f .session/state.txt && mkdir -p out && cat .session/state.txt > out/result.txt")
  ], use_session=SESSION, cascade_session=True),
])
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".to_string(), config.display().to_string());
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("session=remote-or-local"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("out/result.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join(".session/state.txt").exists());
    Ok(())
}
