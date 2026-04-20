#![cfg(target_os = "linux")]

use crate::support;

use support::remote_cli::remote_token;
use support::remote_scan::{CameraFixture, FrameFixture, run_scan, write_scan_fixture};

#[test]
fn remote_scan_can_back_out_of_confirmation_without_writing_inventory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let fixture_path = temp.path().join("scan.toml");
    let token = remote_token("builder-cancel", "http://127.0.0.1:43123", "direct");
    write_scan_fixture(
        &fixture_path,
        &[CameraFixture {
            id: "cam0",
            name: "Desk Camera",
            frames: &[FrameFixture::QrPayload {
                payload: &token,
                width: 192,
            }],
        }],
    )
    .expect("write scan fixture");

    let output = run_scan(&config_root, &fixture_path, "enter,tick,esc,quit").expect("run scan");

    assert!(
        output.status.success(),
        "tak remote scan should allow cancel"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation UI:\n{stdout}"
    );
    assert!(
        stdout.contains("scan cancelled"),
        "missing cancel message:\n{stdout}"
    );
    assert!(
        !config_root.join("tak/remotes.toml").exists(),
        "inventory should stay empty"
    );
}
