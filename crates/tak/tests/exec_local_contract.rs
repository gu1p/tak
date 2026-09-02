//! Black-box contracts for daemon-owned local `tak exec` workflows.

use std::fs;

use anyhow::Result;

use crate::support::exec_daemon::ExecDaemon;
use crate::support::run_tak_output;

#[test]
fn exec_streams_raw_command_output_without_needing_tasks_py() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace)?;
    let daemon = ExecDaemon::spawn(temp.path(), &workspace);
    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--",
            "/bin/sh",
            "-c",
            "printf 'stdout-line\\n'; printf 'stderr-line\\n' >&2; exit 7",
        ],
        daemon.environment(),
    )?;

    assert!(!output.status.success(), "status: {:?}", output.status);
    assert!(String::from_utf8_lossy(&output.stdout).contains("stdout-line"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("stderr-line"));
    Ok(())
}

#[test]
fn exec_honors_cwd_and_env_overrides_in_the_daemon_workspace() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("work"))?;
    let daemon = ExecDaemon::spawn(temp.path(), &workspace);
    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--cwd",
            "work",
            "--env",
            "HELLO=world",
            "--",
            "/bin/sh",
            "-c",
            "printf '%s\\n%s\\n' \"${PWD##*/}\" \"$HELLO\"",
        ],
        daemon.environment(),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.lines().any(|line| line == "work"), "{stdout}");
    assert!(stdout.lines().any(|line| line == "world"), "{stdout}");
    Ok(())
}
