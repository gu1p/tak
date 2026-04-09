use std::path::Path;
use std::process::Output;

pub fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn assert_success_with_log(output: &Output, command: &str, log_path: &Path) {
    let log = std::fs::read_to_string(log_path)
        .unwrap_or_else(|_| format!("missing service log at {}", log_path.display()));
    assert!(
        output.status.success(),
        "{command} should succeed\nstdout:\n{}\nstderr:\n{}\nservice.log:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        log
    );
}
