use crate::support::make_runtime::{install_fake_make, start_local_daemon};
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[cfg(unix)]
#[test]
fn make_runs_a_goal_without_needing_tasks_py() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("Makefile"), "test:\n\t@:\n")?;
    let path = install_fake_make(
        workspace.path(),
        r#"#!/bin/sh
printf 'cwd=%s\n' "$PWD" > make-invocation.txt
for arg in "$@"; do
  printf 'arg=%s\n' "$arg" >> make-invocation.txt
done
"#,
    )?;
    let mut env = BTreeMap::new();
    env.insert("PATH".to_string(), path);
    let _daemon = start_local_daemon(workspace.path(), &mut env);

    let output = run_tak_output(
        workspace.path(),
        &["make", "test", "--pass-env", "PATH"],
        &env,
    )?;

    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let invocation = fs::read_to_string(workspace.path().join("make-invocation.txt"))?;
    assert!(invocation.contains("cwd="), "{invocation}");
    assert!(invocation.contains("arg=test"), "{invocation}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("run_id="));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "info: no Tak execution configuration found for Make goal `test`; running locally outside \
         a container. To run remotely, set `# tak: default.execution=remote` plus a default \
         container image or Dockerfile, add equivalent annotations to this goal, or pass \
         `--remote` with a container source.\n"
    );
    Ok(())
}
