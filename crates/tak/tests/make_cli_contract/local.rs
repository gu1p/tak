use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[cfg(unix)]
#[test]
fn make_runs_a_goal_without_needing_tasks_py() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("Makefile"), "test:\n\t@:\n")?;
    let invocation_log = workspace.path().join("make-invocation.txt");
    let path = install_fake_make(
        workspace.path(),
        r#"#!/bin/sh
printf 'cwd=%s\n' "$PWD" > "$TAK_FAKE_MAKE_LOG"
for arg in "$@"; do
  printf 'arg=%s\n' "$arg" >> "$TAK_FAKE_MAKE_LOG"
done
"#,
    )?;
    let mut env = BTreeMap::new();
    env.insert("PATH".to_string(), path);
    env.insert(
        "TAK_FAKE_MAKE_LOG".to_string(),
        invocation_log.display().to_string(),
    );

    let output = run_tak_output(workspace.path(), &["make", "test"], &env)?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        fs::read_to_string(invocation_log)?,
        format!(
            "cwd={}\narg=test\n",
            workspace.path().canonicalize()?.display()
        )
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "info: no Tak execution configuration found for Make goal `test`; running locally outside \
         a container. To run remotely, set `# tak: default.execution=remote` plus a default \
         container image or Dockerfile, add equivalent annotations to this goal, or pass \
         `--remote` with a container source.\n"
    );
    Ok(())
}
