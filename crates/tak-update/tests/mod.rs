// Integration test aggregator for `tak-update`.
//
// Per the workspace test convention (`autotests = false` + a single `[[test]]`
// pointing here), every test module is declared below. Modules are added as the
// feature's phases land.

mod fixtures;

mod archive_extract_contract;
mod end_to_end_update_behavior;
mod install_target_contract;
mod installer_apply_contract;
mod installer_rollback_contract;
mod latest_tag_parse_contract;
mod sha256_verify_contract;
mod signature_verify_contract;
mod swap_contract;
mod target_detection_contract;
mod update_guard_behavior;
mod ureq_network_smoke;
mod version_compare_contract;
