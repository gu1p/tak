#![cfg(not(target_os = "linux"))]

use crate::support;

use support::remote_scan::run_scan;

#[test]
fn remote_scan_reports_linux_only_support_on_non_linux_hosts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let fixture_path = temp.path().join("scan.toml");

    let output = run_scan(&config_root, &fixture_path, "enter").expect("run scan");

    assert!(
        !output.status.success(),
        "tak remote scan should fail on non-Linux hosts\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("remote scan is currently supported only on Linux"),
        "missing unsupported-platform error\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
