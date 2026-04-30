use crate::support;

use std::fs;
use std::process::Command as StdCommand;

#[test]
fn logs_all_prints_complete_service_log() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(
        state_root.join("service.log"),
        "line-1\nline-2\nline-3\nline-4\n",
    )
    .expect("write service log");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "logs",
            "--state-root",
            &state_root.display().to_string(),
            "--all",
        ])
        .output()
        .expect("run takd logs --all");

    assert!(output.status.success(), "takd logs --all should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "line-1\nline-2\nline-3\nline-4\n");
}
