use crate::support;

use support::installer::{fake_systemctl, run_installer};

#[test]
fn linux_installer_prints_recent_takd_logs_when_token_wait_fails() {
    let (_temp, _home, output) =
        run_installer(fake_systemctl(), &[("TAKD_INSTALLER_FAKE_TOKEN_FAIL", "1")]);

    assert!(!output.status.success(), "installer should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        !combined.contains("takd ready at"),
        "must not report ready before token success:\n{combined}"
    );
    assert!(combined.contains("tor transport is pending"), "{combined}");
    assert!(combined.contains("recent takd logs:"), "{combined}");
    assert!(
        combined.contains("rendezvous circuit timed out"),
        "{combined}"
    );
}
