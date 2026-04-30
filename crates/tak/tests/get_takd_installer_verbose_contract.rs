use crate::support;

use std::fs;
use std::path::PathBuf;

use support::installer::{fake_systemctl, run_installer};

#[test]
fn linux_installer_streams_verbose_readiness_diagnostics_before_token_ready() {
    let (_temp, home, output) = run_installer(
        fake_systemctl(),
        &[
            ("TAKD_INSTALLER_VERBOSE", "1"),
            ("TAKD_INSTALLER_FAKE_PENDING_ATTEMPTS", "1"),
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
    assert!(combined.contains("service_log:"), "{combined}");
    assert!(combined.contains("transport_health:"), "{combined}");
    assert!(combined.contains("readiness snapshot"), "{combined}");
    assert!(combined.contains("transport_state: pending"), "{combined}");
    assert!(combined.contains("transport-health.toml"), "{combined}");
    assert!(
        combined.contains("self-probe connect attempt"),
        "{combined}"
    );
    assert!(combined.contains("Scan this QR code"), "{combined}");
    assert!(home.join(".local/state/takd/service.log").exists());
}

#[test]
fn linux_installer_service_runs_takd_with_trace_logging_by_default() {
    let (_temp, home, output) = run_installer(fake_systemctl(), &[]);

    assert!(output.status.success(), "installer should succeed");
    let unit = fs::read_to_string(home.join(".config/systemd/user/takd.service"))
        .expect("read takd.service");
    assert!(unit.contains("Environment=RUST_LOG=trace"), "{unit}");
    assert!(unit.contains("Environment=RUST_BACKTRACE=1"), "{unit}");
}

#[test]
fn linux_installer_service_trace_level_is_overrideable() {
    let (_temp, home, output) = run_installer(
        fake_systemctl(),
        &[("TAKD_SERVICE_RUST_LOG", "takd=trace,arti_client=trace")],
    );

    assert!(output.status.success(), "installer should succeed");
    let unit = fs::read_to_string(home.join(".config/systemd/user/takd.service"))
        .expect("read takd.service");
    assert!(
        unit.contains("Environment=RUST_LOG=takd=trace,arti_client=trace"),
        "{unit}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn macos_launch_agent_receives_verbose_service_environment() {
    let installer = fs::read_to_string(repo_root().join("get-takd.sh")).expect("read installer");

    assert!(installer.contains("<key>EnvironmentVariables</key>"));
    assert!(installer.contains("<key>RUST_LOG</key><string>${service_rust_log}</string>"));
    assert!(installer.contains("<key>RUST_BACKTRACE</key><string>${service_backtrace}</string>"));
}
