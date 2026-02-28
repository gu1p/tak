//! Black-box E2E contract for CurrentState transfer boundary.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::e2e_harness::{find_free_tcp_port, run_tak_expect_success, spawn_daemon, write_tasks};

#[test]
fn e2e_remote_only_current_state_honors_roots_ignored_include_order() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let remote_port = find_free_tcp_port()?;

    let project_dir = temp.path().join("apps/web/project");
    fs::create_dir_all(project_dir.join("ignored"))?;
    fs::write(project_dir.join("keep.txt"), "keep")?;
    fs::write(project_dir.join("ignored/drop.txt"), "drop")?;
    fs::write(project_dir.join("ignored/bring_back.txt"), "keep")?;

    write_tasks(
        temp.path(),
        &format!(
            r#"
REMOTE = Remote(id="remote-primary", endpoint="http://127.0.0.1:{remote_port}")

SPEC = module_spec(tasks=[
  task(
    "context_boundary",
    steps=[cmd("sh", "-c", "test -f apps/web/project/keep.txt && test -f apps/web/project/ignored/bring_back.txt && test ! -f apps/web/project/ignored/drop.txt")],
    context=CurrentState(
      roots=[path("//apps/web/project")],
      ignored=[path("//apps/web/project/ignored")],
      include=[path("//apps/web/project/ignored/bring_back.txt")],
    ),
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#
        ),
    )?;

    let env = BTreeMap::new();
    let _remote_worker = spawn_daemon(
        temp.path().join("remote-worker.sock"),
        temp.path().join("remote-worker.sqlite"),
        Some(remote_port),
        &env,
    )?;
    let local_daemon = spawn_daemon(
        temp.path().join("local-daemon.sock"),
        temp.path().join("local-daemon.sqlite"),
        None,
        &env,
    )?;

    let run = run_tak_expect_success(
        temp.path(),
        &["run", "apps/web:context_boundary"],
        Some(&local_daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("placement=remote"));
    assert!(run.contains("apps/web:context_boundary: ok"));

    Ok(())
}
