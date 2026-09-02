//! Black-box contract for Make process output and exit-status propagation.

#![cfg(unix)]

use crate::support::make_runtime::{install_fake_make, start_local_daemon};
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
    let _daemon = start_local_daemon(workspace.path(), &mut env);

    let output = run_tak_output(
        workspace.path(),
        &["make", "fail", "--pass-env", "PATH"],
        &env,
    )?;

    assert_eq!(output.status.code(), Some(7), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("make-stdout\n"), "{stdout}");
    assert!(
        stderr.contains("no Tak execution configuration"),
        "{stderr}"
    );
    assert!(stderr.contains("make-stderr\n"), "{stderr}");
    Ok(())
}
