use std::collections::BTreeSet;
use std::fs;

use anyhow::Result;

use crate::support::container_runtime::simulated_container_runtime_env;
use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn container_reuse_cascade_fuses_dependency_closure_into_one_run() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Container.Image("alpine:3.20")
SESSION = session("container", reuse=SessionReuse.Container())
EXEC = Execution.Local(container=RUNTIME, session=SESSION)

SPEC = module_spec(tasks=[
  task("prepare", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf 'prepare\n' > out/order.txt")]),
  task("build", deps=[":prepare"], steps=[cmd("sh", "-c", "printf 'build\n' >> out/order.txt")]),
  task("check", deps=[":build"], steps=[cmd("sh", "-c", "printf 'check\n' >> out/order.txt")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(&workspace);
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("reuse=container"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/order.txt"))?,
        "prepare\nbuild\ncheck\n"
    );

    let task_run_ids = task_run_ids(&stdout);
    assert_eq!(task_run_ids.len(), 1, "stdout:\n{stdout}");
    Ok(())
}

#[test]
fn outer_container_reuse_cascade_absorbs_inner_alias_with_same_execution() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Container.Image("alpine:3.20")
SESSION = session("container", reuse=SessionReuse.Container())
EXEC = Execution.Local(container=RUNTIME, session=SESSION)

SPEC = module_spec(tasks=[
  task("prepare", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf 'prepare\n' > out/nested.txt")]),
  task("check", deps=[":prepare"], steps=[cmd("sh", "-c", "printf 'check\n' >> out/nested.txt")], execution=EXEC, cascade_execution=True),
  task("ci", deps=[":check"], steps=[cmd("sh", "-c", "printf 'ci\n' >> out/nested.txt")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(&workspace);
    let stdout = run_tak_expect_success(&workspace, &["run", "ci"], &env)?;

    assert!(stdout.contains("reuse=container"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/nested.txt"))?,
        "prepare\ncheck\nci\n"
    );
    assert_eq!(task_run_ids(&stdout).len(), 1, "stdout:\n{stdout}");
    Ok(())
}

fn task_run_ids(stdout: &str) -> BTreeSet<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let start = line.find("task_run_id=")? + "task_run_id=".len();
            let rest = &line[start..];
            Some(rest.split(',').next()?.to_string())
        })
        .collect()
}
