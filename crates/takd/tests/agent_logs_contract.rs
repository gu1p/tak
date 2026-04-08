use std::fs;
use std::process::Command as StdCommand;

#[test]
fn logs_tails_requested_number_of_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(
        state_root.join("service.log"),
        "line-1\nline-2\nline-3\nline-4\n",
    )
    .expect("write service log");

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "logs",
            "--state-root",
            &state_root.display().to_string(),
            "--lines",
            "2",
        ])
        .output()
        .expect("run takd logs");

    assert!(output.status.success(), "takd logs should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "line-3\nline-4\n");
}

#[test]
fn logs_reports_missing_service_log_with_actionable_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args(["logs", "--state-root", &state_root.display().to_string()])
        .output()
        .expect("run takd logs");

    assert!(!output.status.success(), "takd logs should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("service log not found"),
        "unexpected stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(&state_root.join("service.log").display().to_string()),
        "missing log path:\n{stderr}"
    );
}

#[test]
fn logs_returns_empty_output_for_empty_service_log() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(state_root.join("service.log"), "").expect("write empty service log");

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args(["logs", "--state-root", &state_root.display().to_string()])
        .output()
        .expect("run takd logs");

    assert!(output.status.success(), "takd logs should succeed");
    assert!(
        output.stdout.is_empty(),
        "expected empty stdout, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn logs_accepts_zero_requested_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    fs::write(state_root.join("service.log"), "line-1\nline-2\n").expect("write service log");

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "logs",
            "--state-root",
            &state_root.display().to_string(),
            "--lines",
            "0",
        ])
        .output()
        .expect("run takd logs");

    assert!(output.status.success(), "takd logs should succeed");
    assert!(
        output.stdout.is_empty(),
        "expected empty stdout, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
