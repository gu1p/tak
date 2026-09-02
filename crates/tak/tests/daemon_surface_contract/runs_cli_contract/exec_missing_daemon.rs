use std::collections::BTreeMap;
use std::fs;

use crate::support::run_tak_output;

#[test]
fn exec_reports_an_actionable_missing_daemon_without_client_execution() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let socket = "../missing-takd.sock";
    let env = BTreeMap::from([("TAKD_SOCKET".into(), socket.into())]);

    let output = run_tak_output(
        &workspace,
        &["exec", "--", "sh", "-c", "touch client-executor-ran"],
        &env,
    )
    .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(stderr.contains(socket), "{stderr}");
    assert!(stderr.contains("takd serve"), "{stderr}");
    assert!(stderr.contains("no client execution fallback"), "{stderr}");
    assert!(!workspace.join("client-executor-ran").exists());
}
