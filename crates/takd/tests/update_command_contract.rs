use crate::support;

use std::process::Command as StdCommand;

#[test]
fn takd_update_help_lists_flags() {
    let output = StdCommand::new(support::takd_bin())
        .args(["update", "--help"])
        .output()
        .expect("run takd update --help");
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--check"), "stdout: {stdout}");
    assert!(stdout.contains("--force"), "stdout: {stdout}");
    assert!(
        stdout.contains("signed GitHub releases"),
        "stdout: {stdout}"
    );
}
