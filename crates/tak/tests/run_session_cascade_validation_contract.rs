use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn cascaded_execution_overrides_child_with_different_explicit_session() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SESSION_A = session("a", reuse=SessionReuse.Workspace())
SESSION_B = session("b", reuse=SessionReuse.Workspace())
EXEC_A = Execution.Local(session=SESSION_A)
EXEC_B = Execution.Local(session=SESSION_B)

SPEC = module_spec(
  tasks=[
    task("child", steps=[cmd("true")], execution=EXEC_B),
    task("check", deps=[":child"], execution=EXEC_A, cascade_execution=True),
  ],
)
SPEC
"#,
    )?;

    let env = BTreeMap::new();
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(
        stdout.contains("//:child: ok") && stdout.contains("session=a"),
        "stdout:\n{stdout}"
    );
    Ok(())
}
