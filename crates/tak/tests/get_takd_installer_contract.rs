mod support;

use std::fs;

use support::installer::{failing_systemctl, fake_systemctl, run_installer};

#[test]
fn linux_installer_bootstraps_takd_user_service_and_prints_token() {
    let (_temp, home, output) = run_installer(fake_systemctl(), &[("TAKD_INSTALL_TEST_MODE", "1")]);

    assert!(
        output.status.success(),
        "installer should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        home.join(".local/bin/takd").exists(),
        "takd should be installed"
    );
    assert!(home.join(".config/systemd/user/takd.service").exists());
    let unit = fs::read_to_string(home.join(".config/systemd/user/takd.service"))
        .expect("read takd.service");
    assert!(
        unit.contains(&format!(
            "ExecStart={} serve --config-root {} --state-root {}",
            home.join(".local/bin/takd").display(),
            home.join(".config/takd").display(),
            home.join(".local/state/takd").display()
        )),
        "unexpected unit file:\n{unit}"
    );
    assert!(
        !unit.contains("StandardOutput=") && !unit.contains("StandardError="),
        "installer should not rely on systemd log redirection:\n{unit}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("takd:v1:"), "missing token:\n{stdout}");
    assert!(stdout.contains(".onion"), "missing onion url:\n{stdout}");
}

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
}
