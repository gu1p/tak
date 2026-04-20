use crate::support;

use std::process::Command as StdCommand;

#[test]
fn status_reports_that_coordination_status_is_unavailable() {
    let output = StdCommand::new(support::tak_bin())
        .arg("status")
        .output()
        .expect("run tak status");

    assert!(!output.status.success(), "tak status should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("coordination status is unavailable in this client-only build"),
        "unexpected stderr:\n{stderr}"
    );
}
