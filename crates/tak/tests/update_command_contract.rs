use crate::support;

use std::process::Command as StdCommand;

#[test]
fn tak_update_help_lists_flags() {
    let output = StdCommand::new(support::tak_bin())
        .args(["update", "--help"])
        .output()
        .expect("run tak update --help");
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--check"), "stdout: {stdout}");
    assert!(stdout.contains("--force"), "stdout: {stdout}");
    assert!(
        stdout.contains("signed GitHub releases"),
        "stdout: {stdout}"
    );
}
