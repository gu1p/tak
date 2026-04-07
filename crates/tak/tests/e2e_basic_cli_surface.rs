//! Black-box E2E contract for core local CLI flow.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn e2e_basic_cli_surface_and_local_dep_chain() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let run_log = temp.path().join("out/run.log");

    write_tasks(
        temp.path(),
        &format!(
            r#"
build = task("build", steps=[cmd("sh", "-c", "mkdir -p out && echo build >> {run_log}")])
test = task("test", deps=[":build"], steps=[cmd("sh", "-c", "echo test >> {run_log}")])
SPEC = module_spec(tasks=[build, test])
SPEC
"#,
            run_log = run_log.display()
        ),
    )?;

    let env = BTreeMap::new();
    let list = run_tak_expect_success(temp.path(), &["list"], &env)?;
    assert!(list.contains("//apps/web:build"));
    assert!(list.contains("//apps/web:test"));

    let explain = run_tak_expect_success(temp.path(), &["explain", "apps/web:test"], &env)?;
    assert!(explain.contains("label: apps/web:test"));
    assert!(explain.contains("deps:"));
    assert!(explain.contains("apps/web:build"));

    let graph = run_tak_expect_success(
        temp.path(),
        &["graph", "apps/web:test", "--format", "dot"],
        &env,
    )?;
    assert!(graph.contains("digraph tak"));
    assert!(graph.contains("\"apps/web:build\" -> \"apps/web:test\""));

    let tree = run_tak_expect_success(temp.path(), &["tree"], &env)?;
    assert!(tree.contains("Tak Tree"));
    assert!(tree.contains("//apps/web:test"));

    let run = run_tak_expect_success(temp.path(), &["run", "apps/web:test"], &env)?;
    assert!(run.contains("apps/web:build: ok"));
    assert!(run.contains("apps/web:test: ok"));

    let lines = fs::read_to_string(&run_log)?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, vec!["build", "test"]);

    Ok(())
}
