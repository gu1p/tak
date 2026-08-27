use super::*;

#[test]
fn linux_installer_falls_back_to_manual_start_without_usable_systemctl_user() {
    let (_temp, home, output) = run_installer(failing_systemctl(), &[]);

    assert!(
        output.status.success(),
        "installer should succeed with manual fallback\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        home.join(".local/bin/takd").exists(),
        "takd should be installed"
    );
    assert!(home.join(".config/systemd/user/takd.service").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("automatic service startup is unavailable"));
    assert!(stdout.contains("takd serve --config-root"));
    assert!(stdout.contains("takd token show --state-root"));
    assert!(
        !stdout.contains("token: "),
        "manual fallback should not print a token:\n{stdout}"
    );
    assert!(
        !stdout.contains("Scan this QR code"),
        "manual fallback should stay plain text:\n{stdout}"
    );
}
