use std::collections::BTreeMap;

use crate::support::run_tak_output;

#[test]
fn an_invalid_run_id_fails_before_contacting_the_daemon_without_echoing_input() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("not-running.sock");
    let invalid_run_id = "s".repeat(129);
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let output = run_tak_output(
        root.path(),
        &["runs", "show", invalid_run_id.as_str()],
        &env,
    )
    .expect("run show with invalid id");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Run ID is invalid"), "{stderr}");
    assert!(stderr.contains("1 to 128 UTF-8 bytes"), "{stderr}");
    assert!(!stderr.contains(&invalid_run_id), "{stderr}");
    assert!(!stderr.contains(&socket.display().to_string()), "{stderr}");
    assert!(!stderr.contains("takd serve"), "{stderr}");
}
