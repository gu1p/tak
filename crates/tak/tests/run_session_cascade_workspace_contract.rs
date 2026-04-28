use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn parent_use_session_cascades_share_workspace_to_dependency_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("rust", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())

SPEC = module_spec(
  tasks=[
    task("build", steps=[cmd("sh", "-c", "mkdir -p .session && echo cached > .session/build.txt")]),
    task("test", deps=[":build"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f .session/build.txt && mkdir -p out && cat .session/build.txt > out/result.txt")]),
    task("check", deps=[":test"], execution=Execution.Session(SESSION, cascade=True)),
  ],
)
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("session=rust"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("reuse=share_workspace"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("out/result.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join(".session/build.txt").exists());
    Ok(())
}
