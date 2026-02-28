//! Black-box E2E contracts for daemon-backed runtime features.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

mod support;
use support::e2e_harness::{run_tak_expect_success, spawn_daemon, write_tasks};

#[test]
fn e2e_daemon_status_and_run_with_needs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let socket = temp.path().join("takd.sock");
    let db = temp.path().join("takd.sqlite");
    let output = temp.path().join("out/limited.txt");

    write_tasks(
        temp.path(),
        &format!(
            r#"
SPEC = module_spec(
  limiters=[resource("cpu", 8, unit="slots", scope=MACHINE)],
  tasks=[
    task(
      "limited",
      needs=[need("cpu", 1, scope=MACHINE)],
      steps=[cmd("sh", "-c", "mkdir -p out && echo daemon-path > {output}")],
    )
  ],
)
SPEC
"#,
            output = output.display()
        ),
    )?;

    let env = BTreeMap::new();
    let daemon = spawn_daemon(socket.clone(), db, None, &env)?;

    let status = run_tak_expect_success(temp.path(), &["status"], Some(&daemon.socket_path), &env)?;
    assert!(status.contains("active_leases:"));
    assert!(status.contains("pending_requests:"));

    let daemon_status = run_tak_expect_success(
        temp.path(),
        &["daemon", "status"],
        Some(&daemon.socket_path),
        &env,
    )?;
    assert!(daemon_status.contains("active_leases:"));
    assert!(daemon_status.contains("pending_requests:"));

    let run = run_tak_expect_success(
        temp.path(),
        &["run", "apps/web:limited"],
        Some(&daemon.socket_path),
        &env,
    )?;
    assert!(run.contains("apps/web:limited: ok"));
    assert_eq!(fs::read_to_string(output)?.trim(), "daemon-path");

    Ok(())
}
