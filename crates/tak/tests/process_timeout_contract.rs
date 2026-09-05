use super::support::run_timeout::output_with_timeout;
use anyhow::Result;
use std::process::Command;
use std::time::{Duration, Instant};

#[cfg(unix)]
#[test]
fn stalled_example_clients_are_terminated_at_their_deadline() -> Result<()> {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "while :; do :; done"]);
    let started = Instant::now();

    let error = output_with_timeout(command, Duration::from_millis(50))
        .expect_err("the stalled command must time out");

    assert!(
        error.to_string().contains("timed out after 50ms"),
        "unexpected timeout error: {error:#}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the command deadline must bound the wait"
    );
    Ok(())
}

#[test]
fn bounded_commands_preserve_output_and_nonzero_exit_status() -> Result<()> {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf stdout; printf stderr >&2; exit 7"]);
    let output = output_with_timeout(command, Duration::from_secs(5))?;
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"stdout");
    assert_eq!(output.stderr, b"stderr");
    Ok(())
}

#[test]
fn bounded_commands_capture_output_larger_than_pipe_capacity() -> Result<()> {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "head -c 262144 /dev/zero"]);
    let output = output_with_timeout(command, Duration::from_secs(5))?;
    assert!(output.status.success());
    assert_eq!(output.stdout, vec![0; 262144]);
    Ok(())
}
