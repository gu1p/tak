use std::process::Command as StdCommand;

use crate::support;

use support::takd_bin;

#[test]
fn takd_daemon_serve_is_not_part_of_the_cli_contract() {
    let output = StdCommand::new(takd_bin())
        .args(["daemon", "serve", "--help"])
        .output()
        .expect("run takd daemon serve help");

    assert!(
        !output.status.success(),
        "takd daemon serve help should fail\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand")
            || stderr.contains("unexpected argument")
            || stderr.contains("Usage:"),
        "takd daemon serve should be rejected by clap\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
