use std::time::Duration;

use super::runtime::RemoteRuntimeConfig;

#[test]
fn default_remote_client_stale_ttl_outlasts_tor_event_reconnect_budget() {
    let runtime = RemoteRuntimeConfig::for_tests();

    assert!(runtime.remote_client_stale_ttl() >= Duration::from_secs(450));
    assert!(runtime.remote_client_stale_ttl() < runtime.remote_cleanup_ttl());
}
