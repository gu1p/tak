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

#[test]
fn host_baseline_sampling_runs_on_real_nodes_and_skips_test_roots() {
    let production = RemoteRuntimeConfig::from_environment(|_| None, std::env::temp_dir(), false);
    let test_root = RemoteRuntimeConfig::from_environment(|_| None, std::env::temp_dir(), true);
    let overridden = RemoteRuntimeConfig::from_environment(
        |name| (name == "TAKD_HOST_BASELINE_SAMPLE_MS").then(|| "1250".into()),
        std::env::temp_dir(),
        false,
    );

    assert_eq!(
        production.host_baseline_sample_duration(),
        Duration::from_secs(5)
    );
    assert_eq!(test_root.host_baseline_sample_duration(), Duration::ZERO);
    assert_eq!(
        overridden.host_baseline_sample_duration(),
        Duration::from_millis(1250)
    );
}

#[test]
fn host_baseline_sampling_can_be_disabled_explicitly() {
    let runtime = RemoteRuntimeConfig::from_environment(
        |name| (name == "TAKD_HOST_BASELINE_SAMPLE_MS").then(|| "0".into()),
        std::env::temp_dir(),
        false,
    );

    assert_eq!(runtime.host_baseline_sample_duration(), Duration::ZERO);
}

#[test]
fn simulated_host_defaults_to_deterministic_resource_accounting() {
    let runtime = RemoteRuntimeConfig::from_environment(
        |name| (name == "TAK_TEST_HOST_PLATFORM").then(|| "other".into()),
        std::env::temp_dir(),
        false,
    );

    assert_eq!(runtime.host_baseline_sample_duration(), Duration::ZERO);
    assert!(runtime.ignore_host_usage_for_tests());
}
