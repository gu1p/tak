//! Black-box contract for `tak exec --local-no-container`.

use crate::support::exec_daemon::ExecDaemon;
use crate::support::run_tak_output;

use std::fs;

use anyhow::Result;

#[test]
fn exec_supports_local_no_container_host_execution() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace)?;
    let mut daemon = ExecDaemon::spawn(temp.path(), &workspace);
    daemon
        .environment_mut()
        .insert("TAK_RUNTIME_SOURCE".into(), "none".into());
    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--local-no-container",
            "--pass-env",
            "TAK_RUNTIME_SOURCE",
            "--",
            "/bin/sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        daemon.environment(),
    )?;

    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == "none")
    );
    Ok(())
}
