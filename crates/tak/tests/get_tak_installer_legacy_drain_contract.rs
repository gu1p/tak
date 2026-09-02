use super::installer_drain::{Installer, assert_drain_contract};

#[test]
fn release_installer_checks_legacy_work_before_replacing_binaries() {
    assert_drain_contract(Installer::ReleaseBundle);
}
