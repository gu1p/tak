use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_failure, write_tasks};

#[test]
fn cascaded_session_rejects_child_with_different_explicit_session() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION_A = session("a", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SESSION_B = session("b", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())

SPEC = module_spec(
  sessions=[SESSION_A, SESSION_B],
  tasks=[
    task("child", steps=[cmd("true")], execution=Execution.Session("b")),
    task("check", deps=[":child"], execution=Execution.Session("a", cascade=True)),
  ],
)
SPEC
"#,
    )?;

    let env = BTreeMap::new();
    let (_stdout, stderr) = run_tak_expect_failure(&workspace, &["run", "check"], &env)?;

    assert!(
        stderr.contains("session cascade conflict")
            && stderr.contains("//:child")
            && stderr.contains("`a`")
            && stderr.contains("`b`"),
        "stderr:\n{stderr}"
    );
    Ok(())
}
