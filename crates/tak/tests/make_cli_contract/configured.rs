use crate::support::make_runtime::install_fake_make;
use crate::support::run_tak_output;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

#[cfg(unix)]
#[test]
fn explicit_local_host_configuration_suppresses_the_fallback_notice() -> Result<()> {
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("Makefile"), "test:\n\t@:\n")?;
    let path = install_fake_make(workspace.path(), "#!/bin/sh\nexit 0\n")?;
    let env = BTreeMap::from([("PATH".to_string(), path)]);

    let output = run_tak_output(
        workspace.path(),
        &["make", "--local-no-container", "test"],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    Ok(())
}
