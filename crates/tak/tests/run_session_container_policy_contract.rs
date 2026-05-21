use std::collections::BTreeSet;
use std::fs;

use anyhow::Result;

use crate::support::container_runtime::simulated_container_runtime_env;
use crate::support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn container_reuse_cascade_honors_dependency_retry_policy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512))
SESSION = session("container", reuse=SessionReuse.Container())
EXEC = Execution.Local(container=RUNTIME, session=SESSION)

SPEC = module_spec(tasks=[
  task("flaky", retry=retry(attempts=2, on_exit=[42], backoff=fixed(0)), steps=[cmd("sh", "-c", "mkdir -p out && if [ -f out/flaky.seen ]; then printf 'recovered\n' > out/flaky.txt; exit 0; else touch out/flaky.seen; exit 42; fi")]),
  task("check", deps=[":flaky"], steps=[cmd("sh", "-c", "printf 'check\n' >> out/flaky.txt")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(&workspace);
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert_eq!(
        fs::read_to_string(workspace.join("out/flaky.txt"))?,
        "recovered\ncheck\n"
    );
    assert_eq!(task_run_ids(&stdout).len(), 1, "stdout:\n{stdout}");
    Ok(())
}

#[test]
fn container_reuse_cascade_honors_dependency_timeout_policy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512))
SESSION = session("container", reuse=SessionReuse.Container())
EXEC = Execution.Local(container=RUNTIME, session=SESSION)

SPEC = module_spec(tasks=[
  task("slow", timeout_s=1, steps=[cmd("sh", "-c", "sleep 2 && mkdir -p out && printf 'slow\n' > out/slow.txt")]),
  task("check", deps=[":slow"], timeout_s=10, steps=[cmd("sh", "-c", "mkdir -p out && printf 'check\n' > out/check.txt")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(&workspace);
    let (_stdout, _stderr) = run_tak_expect_failure(&workspace, &["run", "check"], &env)?;

    assert!(!workspace.join("out/slow.txt").exists());
    assert!(!workspace.join("out/check.txt").exists());
    Ok(())
}

#[test]
fn container_reuse_cascade_outputs_reflect_final_workspace_state() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512))
SESSION = session("container", reuse=SessionReuse.Container())
EXEC = Execution.Local(container=RUNTIME, session=SESSION)

SPEC = module_spec(tasks=[
  task("prepare", outputs=[path("out")], steps=[cmd("sh", "-c", "mkdir -p out && printf 'prepare\n' > out/state.txt")]),
  task("check", deps=[":prepare"], steps=[cmd("sh", "-c", "printf 'check\n' > out/state.txt")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    )?;

    let env = simulated_container_runtime_env(&workspace);
    run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert_eq!(
        fs::read_to_string(workspace.join("out/state.txt"))?,
        "check\n"
    );
    Ok(())
}

fn task_run_ids(stdout: &str) -> BTreeSet<String> {
    stdout
        .lines()
        .filter_map(|line| line.split_once("task_run_id=").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split(',').next().map(str::to_string))
        .collect()
}
