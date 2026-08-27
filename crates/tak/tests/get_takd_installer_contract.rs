use crate::support;

use std::fs;
use std::path::PathBuf;

use support::installer::{failing_systemctl, fake_systemctl, run_installer};

mod manual_fallback;

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
    assert!(
        unit.contains("Environment=RUST_LOG=info"),
        "installed services must not fill disks with trace logging by default:\n{unit}"
    );
    assert!(
        unit.contains("OOMPolicy=continue"),
        "one workload failure must not let systemd terminate the worker service:\n{unit}"
    );
    assert!(
        !unit.contains("TAKD_REQUIRE_WORKLOAD_FENCE")
            && !unit.contains("ManagedOOMPreference=")
            && !unit.contains("Delegate="),
        "the default service must not install a cgroup memory cap that can kill task containers:\n{unit}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("takd:tor:"),
        "missing tor invite:\n{stdout}"
    );
    assert!(stdout.contains(".onion"), "missing onion url:\n{stdout}");
    assert!(
        stdout.contains("Scan this QR code"),
        "missing QR onboarding label:\n{stdout}"
    );
    assert!(
        stdout.contains("[tor] Waiting for readiness"),
        "installer should show the long Tor-readiness wait phase:\n{stdout}"
    );
    assert!(
        stdout.lines().filter(|line| line.contains('█')).count() >= 4,
        "missing QR block render:\n{stdout}"
    );
}

#[test]
fn linux_installer_download_uses_visible_progress() {
    let installer = fs::read_to_string(repo_root().join("get-takd.sh")).expect("read installer");

    assert!(
        installer.contains("curl -fL --progress-bar -o"),
        "installer release download must show progress during large takd downloads:\n{installer}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
