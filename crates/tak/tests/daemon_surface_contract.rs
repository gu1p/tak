use std::process::Command as StdCommand;

#[test]
fn daemon_subcommand_is_removed() {
    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
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
