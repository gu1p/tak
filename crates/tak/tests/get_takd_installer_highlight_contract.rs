use crate::support;

use support::installer::{fake_systemctl, run_installer};

#[test]
fn linux_installer_shows_curated_highlights_without_streaming_raw_logs() {
    let (_temp, home, output) = run_installer(
        fake_systemctl(),
        &[
            ("TAKD_INSTALLER_FAKE_PENDING_ATTEMPTS", "2"),
            ("TAKD_WAIT_POLL_SECS", "1"),
        ],
    );

    assert!(
        output.status.success(),
        "installer should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(combined.contains("[download]"), "{combined}");
    assert!(combined.contains("[install]"), "{combined}");
    assert!(combined.contains("[service]"), "{combined}");
    assert!(
        combined.contains("[tor] Waiting for readiness"),
        "{combined}"
    );
    assert!(combined.contains("[tor] pending:"), "{combined}");
    assert!(combined.contains("[ready] takd ready at"), "{combined}");
    assert!(combined.contains("Full logs:"), "{combined}");
    assert!(combined.contains("takd logs --all"), "{combined}");
    assert!(combined.contains("Scan this QR code"), "{combined}");
    assert!(combined.contains("takd:tor:"), "{combined}");
    assert!(home.join(".local/state/takd/service.log").exists());

    assert!(
        !combined.contains("streaming takd service log with tail -F"),
        "{combined}"
    );
    assert!(!combined.contains("readiness snapshot"), "{combined}");
    assert!(
        !combined.contains("takd readiness token attempt"),
        "{combined}"
    );
    assert!(
        !combined.contains("takd token is not ready yet"),
        "{combined}"
    );
    assert!(!combined.contains("recent takd logs:"), "{combined}");
    assert_eq!(combined.matches("[tor] pending:").count(), 1, "{combined}");
}
