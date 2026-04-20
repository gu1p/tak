use std::process::Stdio;

use anyhow::Result;

use crate::support;

use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::streaming::{run_streaming_process_and_capture, write_remote_waiting_tasks};
use support::tor_smoke::{tak_command, takd_bin};

#[test]
fn run_reports_remote_progress_on_stderr_while_waiting_for_logs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    write_remote_waiting_tasks(&workspace_root)?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "remote-progress");
    let _agent = spawn_direct_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut command = tak_command(&workspace_root, &roots.client_config_root);
    command
        .env("TAK_TEST_REMOTE_WAIT_HEARTBEAT_MS", "1000")
        .args(["run", "//:remote_wait"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let (stdout, stderr) = run_streaming_process_and_capture(command)?;
    assert!(stdout.contains("remote-stdout\n"), "stdout:\n{stdout}");
    assert!(stdout.contains("//:remote_wait: ok"), "stdout:\n{stdout}");
    assert!(stderr.contains("probing remote node"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("staging remote workspace"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("submitting to remote node"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("waiting for remote output from"),
        "stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("waiting for remote activity"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("remote task still running on"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("jobs=") && stderr.contains("cpu=") && stderr.contains("ram="),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("remote-stderr\n"), "stderr:\n{stderr}");
    assert!(!stdout.contains("probing remote node"), "stdout:\n{stdout}");
    Ok(())
}
