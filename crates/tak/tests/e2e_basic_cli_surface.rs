//! Black-box E2E contract for core local CLI flow.

use std::fs;

use anyhow::Result;

#[allow(dead_code)]
use crate::support;
use support::exec_daemon::ExecDaemon;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn e2e_basic_cli_surface_and_local_dep_chain() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    let run_log = workspace.join("out/run.log");

    write_tasks(
        &workspace,
        r#"
build = task("build", outputs=[path("out/run.log")], steps=[cmd("sh", "-c", "mkdir -p out && echo build > out/run.log")])
test = task("test", deps=[":build"], outputs=[path("out/run.log")], steps=[cmd("sh", "-c", "echo test >> out/run.log")])
SPEC = module_spec(spec_version=2, tasks=[build, test])
SPEC
"#,
    )?;

    let daemon = ExecDaemon::spawn(temp.path(), &workspace);
    let env = daemon.environment();
    let list = run_tak_expect_success(&workspace, &["list"], env)?;
    assert!(list.contains("//:build"));
    assert!(list.contains("//:test"));

    let explain = run_tak_expect_success(&workspace, &["explain", "//:test"], env)?;
    assert!(explain.contains("label: //:test"));
    assert!(explain.contains("deps:"));
    assert!(explain.contains("//:build"));

    let graph = run_tak_expect_success(&workspace, &["graph", "//:test", "--format", "dot"], env)?;
    assert!(graph.contains("digraph tak"));
    assert!(graph.contains("\"//:build\" -> \"//:test\""));

    let tree = run_tak_expect_success(&workspace, &["tree"], env)?;
    assert!(tree.contains("Tak Tree"));
    assert!(tree.contains("//:test"));

    let run = run_tak_expect_success(&workspace, &["run", "//:test"], env)?;
    assert!(run.contains("succeeded tasks=//:build"), "run:\n{run}");
    assert!(run.contains("succeeded tasks=//:test"), "run:\n{run}");

    let lines = fs::read_to_string(&run_log)?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, vec!["build", "test"]);

    Ok(())
}
