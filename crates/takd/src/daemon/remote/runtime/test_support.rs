use std::time::Duration;

use super::{
    DEFAULT_CONTAINER_CPU_CORES, DEFAULT_CONTAINER_MEMORY_MB, DEFAULT_REMOTE_CLEANUP_INTERVAL_MS,
    DEFAULT_REMOTE_CLEANUP_TTL_MS, DEFAULT_WORKER_CACHE_BUDGET_BYTES, MemoryPressureSettings,
    RemoteRuntimeConfig,
};

impl RemoteRuntimeConfig {
    pub(crate) fn isolated_for_test() -> Self {
        Self {
            explicit_remote_exec_root: None,
            temp_dir: std::env::temp_dir(),
            docker_host: Some("unix:///nonexistent/takd-tests-isolated-docker.sock".to_string()),
            use_temp_dir_default_exec_root: true,
            remote_cleanup_ttl: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_TTL_MS),
            remote_cleanup_interval: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_INTERVAL_MS),
            worker_cache_budget_bytes: DEFAULT_WORKER_CACHE_BUDGET_BYTES,
            memory_pressure: MemoryPressureSettings::defaults(),
            admission_oversubscribe_x: 1,
            default_container_cpu_cores: DEFAULT_CONTAINER_CPU_CORES,
            default_container_memory_mb: DEFAULT_CONTAINER_MEMORY_MB,
            memory_pressure_enabled: false,
            resource_sample_interval: Duration::from_millis(
                super::DEFAULT_RESOURCE_SAMPLE_INTERVAL_MS,
            ),
            host_baseline_sample_duration: Duration::ZERO,
            ignore_host_usage_for_tests: true,
            test_memory_signal_path: None,
        }
    }
}
