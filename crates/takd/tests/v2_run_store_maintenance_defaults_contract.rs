use std::time::Duration;

use takd::RunStoreMaintenanceConfig;

#[test]
fn maintenance_defaults_are_seven_days_thirty_days_and_twenty_gibibytes() {
    let config = RunStoreMaintenanceConfig::default();
    assert_eq!(
        config.terminal_payload_retention,
        Duration::from_secs(7 * 86_400)
    );
    assert_eq!(
        config.terminal_metadata_retention,
        Duration::from_secs(30 * 86_400)
    );
    assert_eq!(
        config.workspace_path_blob_budget_bytes,
        20 * 1024 * 1024 * 1024
    );
}
