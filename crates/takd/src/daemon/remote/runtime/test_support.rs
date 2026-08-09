use std::path::PathBuf;
use std::time::Duration;

use super::{
    DEFAULT_REMOTE_CLEANUP_INTERVAL_MS, DEFAULT_REMOTE_CLEANUP_TTL_MS,
    DEFAULT_REMOTE_CLIENT_STALE_TTL_MS, DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS,
    MemoryPressureSettings, RemoteRuntimeConfig,
};

impl RemoteRuntimeConfig {
    pub(crate) fn isolated_for_test() -> Self {
        Self {
            explicit_remote_exec_root: None,
            temp_dir: std::env::temp_dir(),
            docker_host: Some("unix:///nonexistent/takd-tests-isolated-docker.sock".to_string()),
            podman_socket: None,
            runtime_dir: None,
            uid: None,
            use_temp_dir_default_exec_root: true,
            skip_exec_root_probe: false,
            remote_cleanup_ttl: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_TTL_MS),
            remote_cleanup_interval: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_INTERVAL_MS),
            remote_client_stale_ttl: Duration::from_millis(DEFAULT_REMOTE_CLIENT_STALE_TTL_MS),
            remote_client_watchdog_interval: Duration::from_millis(
                DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS,
            ),
            memory_pressure: MemoryPressureSettings::defaults(),
            admission_oversubscribe_x: 1,
            memory_pressure_enabled: false,
        }
    }

    pub(crate) fn isolated_with_temp_dir_for_test(temp_dir: impl Into<PathBuf>) -> Self {
        Self {
            temp_dir: temp_dir.into(),
            ..Self::isolated_for_test()
        }
    }
}
