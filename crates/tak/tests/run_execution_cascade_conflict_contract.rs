use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::{run_tak_expect_failure, write_tasks};

#[test]
fn overlapping_execution_cascades_with_different_executions_fail_before_running() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"CONTAINER = Container.Image("alpine:3.20")
FAST = Execution.Local()
REMOTE = Execution.Remote(container=CONTAINER)

SPEC = module_spec(tasks=[
  task("shared", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && echo ran > out/shared.txt")]),
  task("left", deps=[":shared"], execution=FAST, cascade_execution=True),
  task("right", deps=[":shared"], execution=REMOTE, cascade_execution=True),
])
SPEC
"#,
    )?;

    let (_stdout, stderr) =
        run_tak_expect_failure(&workspace, &["run", "left", "right"], &BTreeMap::new())?;

    assert!(
        stderr.contains("execution cascade conflict"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("//:shared"), "stderr:\n{stderr}");
    assert!(!workspace.join("out/shared.txt").exists());
    Ok(())
}
