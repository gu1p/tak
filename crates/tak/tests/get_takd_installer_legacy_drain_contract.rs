use super::installer_drain::{Installer, assert_drain_contract};

#[test]
fn agent_installer_checks_legacy_work_before_replacing_takd() {
    assert_drain_contract(Installer::Agent);
}
