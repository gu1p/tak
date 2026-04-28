use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn use_session_rejects_removed_string_session_name() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], execution=Execution.Session("missing"))])
SPEC
"#,
    )?;

    let env = BTreeMap::new();
    let (_stdout, stderr) = run_tak_expect_failure(&workspace, &["run", "check"], &env)?;

    assert!(
        stderr.contains("Execution.Session(...) expects a session(...) object, not a string"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docs_dump_includes_session_dsl_surface() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let env = BTreeMap::new();
    let output = run_tak_expect_success(temp.path(), &["docs", "dump"], &env)?;

    for token in [
        "SessionReuse.Workspace",
        "SessionReuse.Paths",
        "Execution.Session",
        "cascade",
        "SessionLifetime.PerRun",
    ] {
        assert!(output.contains(token), "missing {token} in docs:\n{output}");
    }
    Ok(())
}
