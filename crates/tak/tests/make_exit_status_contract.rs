//! Black-box contract for Make process output and exit-status propagation.

#![cfg(unix)]

use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn make_preserves_output_streams_and_nonzero_exit_status() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("Makefile"), "fail:\n\t@:\n")?;

    let path = install_fake_make(
        workspace.path(),
        "#!/bin/sh\nprintf 'make-stdout\\n'\nprintf 'make-stderr\\n' >&2\nexit 7\n",
    )?;
    let mut env = BTreeMap::new();
    env.insert("PATH".to_string(), path);

    let output = run_tak_output(workspace.path(), &["make", "fail"], &env)?;

    assert_eq!(output.status.code(), Some(7), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "make-stdout\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "info: no Tak execution configuration found for Make goal `fail`; running locally outside \
         a container. To run remotely, set `# tak: default.execution=remote` plus a default \
         container image or Dockerfile, add equivalent annotations to this goal, or pass \
         `--remote` with a container source.\n\
         make-stderr\n"
    );
    Ok(())
}
