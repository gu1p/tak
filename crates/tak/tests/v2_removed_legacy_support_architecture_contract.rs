use std::path::Path;

#[test]
fn client_v1_http_fallback_helpers_stay_removed() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for removed in [
        "support/auth_fallback_servers.rs",
        "support/auth_fallback_servers/request_handlers.rs",
        "support/remote_status.rs",
        "support/remote_status/value.rs",
    ] {
        assert!(
            !tests.join(removed).exists(),
            "legacy client fallback helper remains: {removed}"
        );
    }
    for rejection in [
        "remote_cli_v1_upgrade_contract.rs",
        "daemon_surface_contract/runs_cli_contract/v1_no_retry.rs",
    ] {
        assert!(
            tests.join(rejection).exists(),
            "explicit v1 rejection contract was removed: {rejection}"
        );
    }

    let remote_add = std::fs::read_to_string(tests.join("support/remote_add.rs")).unwrap();
    assert!(!remote_add.contains("spawn_node_info_probe"));
    assert!(!remote_add.contains("/v1/"));
}
