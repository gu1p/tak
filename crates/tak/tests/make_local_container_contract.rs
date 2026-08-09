//! Black-box contract for explicit local Make container execution.

#![cfg(unix)]

use crate::support::container_runtime::simulated_container_runtime_env;
use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::fs;

use anyhow::Result;

#[test]
fn make_cli_can_force_a_local_image_container() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("Makefile"), "check:\n\t@:\n")?;
    let mut env = simulated_container_runtime_env(workspace.path());
    let path = install_fake_make(
        workspace.path(),
        "#!/bin/sh\nprintf 'goal=%s\\nsource=%s\\nimage=%s\\n' \
         \"$1\" \"$TAK_RUNTIME_SOURCE\" \"$TAK_CONTAINER_IMAGE\"\n",
    )?;
    env.insert("PATH".to_string(), path);

    let output = run_tak_output(
        workspace.path(),
        &[
            "make",
            "--local",
            "--container",
            "--container-image",
            "alpine:3.20",
            "check",
        ],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "goal=check\nsource=image\nimage=alpine:3.20\n"
    );
    Ok(())
}
