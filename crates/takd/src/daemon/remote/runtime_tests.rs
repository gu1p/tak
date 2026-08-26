use std::time::Duration;

use super::runtime::RemoteRuntimeConfig;

#[test]
fn default_remote_client_stale_ttl_outlasts_tor_event_reconnect_budget() {
    let runtime = RemoteRuntimeConfig::isolated_for_test();

    assert!(runtime.remote_client_stale_ttl() >= Duration::from_secs(450));
    assert!(runtime.remote_client_stale_ttl() < runtime.remote_cleanup_ttl());
}

#[test]
fn remote_resource_defaults_are_safe_and_operator_overridable() {
    let defaults = RemoteRuntimeConfig::from_environment(|_| None, std::env::temp_dir(), true);
    assert_eq!(defaults.default_container_cpu_cores(), 4.0);
    assert_eq!(defaults.default_container_memory_mb(), 8192);
    assert_eq!(defaults.admission_oversubscribe_x(), 1);

    let overridden = RemoteRuntimeConfig::from_environment(
        |name| match name {
            "TAKD_DEFAULT_CONTAINER_CPU_CORES" => Some("2.5".into()),
            "TAKD_DEFAULT_CONTAINER_MEMORY_MB" => Some("3072".into()),
            "TAKD_ADMISSION_OVERSUBSCRIBE_X" => Some("3".into()),
            _ => None,
        },
        std::env::temp_dir(),
        true,
    );
    assert_eq!(overridden.default_container_cpu_cores(), 2.5);
    assert_eq!(overridden.default_container_memory_mb(), 3072);
    assert_eq!(overridden.admission_oversubscribe_x(), 3);
}
