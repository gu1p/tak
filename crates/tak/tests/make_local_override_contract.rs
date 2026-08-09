//! Black-box contract for overriding Makefile-authored execution locally.

#![cfg(unix)]

use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[test]
fn local_no_container_overrides_remote_image_annotation() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(
        workspace.path().join("Makefile"),
        "# tak: execution=remote\n\
         # tak: container-image=alpine:3.20\n\
         check:\n\
         \t@:\n",
    )?;
    let path = install_fake_make(
        workspace.path(),
        "#!/bin/sh\nprintf 'goal=%s\\nsource=%s\\n' \"$1\" \"$TAK_RUNTIME_SOURCE\"\n",
    )?;
    let env = BTreeMap::from([
        ("PATH".to_string(), path),
        ("TAK_RUNTIME_SOURCE".to_string(), "host".to_string()),
    ]);

    let output = run_tak_output(
        workspace.path(),
        &["make", "--local-no-container", "check"],
        &env,
    )?;

    assert!(
        output.status.success(),
        "status: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "goal=check\nsource=host\n"
    );
    Ok(())
}
