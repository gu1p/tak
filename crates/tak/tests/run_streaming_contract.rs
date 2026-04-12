//! Black-box contract for live task stdout/stderr streaming.

use std::process::{Command as StdCommand, Stdio};

use anyhow::Result;

mod support;

use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::streaming::{
    run_streaming_process_and_capture, write_local_streaming_tasks, write_remote_streaming_tasks,
};
use support::tor_smoke::{tak_command, takd_bin};

#[test]
fn run_streams_local_stdout_and_stderr_before_summary() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_local_streaming_tasks(temp.path())?;

    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command
        .current_dir(temp.path())
        .args(["run", "//:stream_local"])
        .env("TAKD_SOCKET", temp.path().join(".missing-takd.sock"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let (stdout, stderr) = run_streaming_process_and_capture(command)?;
    assert!(stdout.contains("local-stdout\n"));
    assert!(stderr.contains("local-stderr\n"));
    assert!(stdout.contains("//:stream_local: ok"));
    assert!(stdout.find("local-stdout\n") < stdout.find("//:stream_local: ok"));
    assert!(!stderr.contains("probing remote node"));
    Ok(())
}

#[test]
fn run_streams_remote_stdout_and_stderr_before_summary() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    write_remote_streaming_tasks(&workspace_root)?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "remote-streamer");
    let _agent = spawn_direct_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut command = tak_command(&workspace_root, &roots.client_config_root);
    command
        .args(["run", "//:remote_stream"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let (stdout, stderr) = run_streaming_process_and_capture(command)?;
    assert!(stdout.contains("remote-stdout\n"));
    assert!(stderr.contains("remote-stderr\n"));
    assert!(stdout.contains("//:remote_stream: ok"));
    assert!(stdout.find("remote-stdout\n") < stdout.find("//:remote_stream: ok"));
    assert!(!stdout.contains("probing remote node"));
    Ok(())
}
