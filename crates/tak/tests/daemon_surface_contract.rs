use crate::support;

use std::process::Command as StdCommand;

#[path = "daemon_surface_contract/runs_cli_contract.rs"]
mod runs_cli_contract;
#[path = "daemon_surface_contract/v2_authored_spec_contract.rs"]
mod v2_authored_spec_contract;

#[test]
fn daemon_subcommand_is_removed() {
    let output = StdCommand::new(support::tak_bin())
        .args(["daemon", "start"])
        .output()
        .expect("run tak daemon start");

    assert!(!output.status.success(), "daemon command should be removed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand")
            || stderr.contains("unknown subcommand")
            || stderr.contains("unexpected argument"),
        "expected clap to reject removed daemon command\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr
    );
}
