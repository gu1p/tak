use std::path::PathBuf;
use std::time::Duration;

mod config_accessors;
mod env_parse;
mod memory_pressure_settings;
#[cfg(test)]
mod test_support;

use env_parse::{
    bool_from_env, duration_from_env, f64_from_env, optional_trimmed_env, u64_from_env,
};
pub(crate) use memory_pressure_settings::MemoryPressureSettings;

const DEFAULT_REMOTE_CLEANUP_TTL_MS: u64 = 15 * 60 * 1000;
const DEFAULT_REMOTE_CLEANUP_INTERVAL_MS: u64 = 60 * 1000;
const DEFAULT_REMOTE_CLIENT_STALE_TTL_MS: u64 = 600 * 1000;
const DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS: u64 = 1000;
const DEFAULT_RESOURCE_SAMPLE_INTERVAL_MS: u64 = 250;
const DEFAULT_HOST_BASELINE_SAMPLE_MS: u64 = 5_000;
const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";

const DEFAULT_ADMISSION_OVERSUBSCRIBE_X: u64 = 1;
const DEFAULT_CONTAINER_CPU_CORES: f64 = 4.0;
const DEFAULT_CONTAINER_MEMORY_MB: u64 = 8192;

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteRuntimeConfig {
    explicit_remote_exec_root: Option<PathBuf>,
    temp_dir: PathBuf,
    docker_host: Option<String>,
    podman_socket: Option<String>,
    runtime_dir: Option<String>,
    uid: Option<String>,
    use_temp_dir_default_exec_root: bool,
    skip_exec_root_probe: bool,
    remote_cleanup_ttl: Duration,
    remote_cleanup_interval: Duration,
    remote_client_stale_ttl: Duration,
    remote_client_watchdog_interval: Duration,
    memory_pressure: MemoryPressureSettings,
    admission_oversubscribe_x: u64,
    default_container_cpu_cores: f64,
    default_container_memory_mb: u64,
    memory_pressure_enabled: bool,
    resource_sample_interval: Duration,
    host_baseline_sample_duration: Duration,
    ignore_host_usage_for_tests: bool,
    test_memory_signal_path: Option<PathBuf>,
}

impl RemoteRuntimeConfig {
    pub fn from_env() -> Self {
        Self::from_environment(|name| std::env::var(name).ok(), std::env::temp_dir(), false)
    }

    /// Builds a runtime configuration from an immutable environment snapshot.
    ///
    /// `from_env` uses this same path with the process environment. Embedders
    /// can provide an already-captured source when process-wide mutation is not
    /// safe, such as when constructing multiple in-process daemon contexts.
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use takd::RemoteRuntimeConfig;
    ///
    /// let config = RemoteRuntimeConfig::from_environment(
    ///     |_| None,
    ///     PathBuf::from("/tmp"),
    ///     false,
    /// );
    /// # let _ = config;
    /// ```
    pub fn from_environment(
        read_env: impl Fn(&str) -> Option<String>,
        temp_dir: PathBuf,
        use_temp_dir_default_exec_root: bool,
    ) -> Self {
        let simulated_host = read_env("TAK_TEST_HOST_PLATFORM").is_some();
        let host_baseline_default_ms = if use_temp_dir_default_exec_root || simulated_host {
            0
        } else {
            DEFAULT_HOST_BASELINE_SAMPLE_MS
        };
        Self {
            explicit_remote_exec_root: optional_trimmed_env(&read_env, "TAKD_REMOTE_EXEC_ROOT")
                .map(PathBuf::from),
            temp_dir,
            docker_host: optional_trimmed_env(&read_env, "DOCKER_HOST"),
            podman_socket: optional_trimmed_env(&read_env, "TAK_PODMAN_SOCKET"),
            runtime_dir: optional_trimmed_env(&read_env, "XDG_RUNTIME_DIR"),
            uid: optional_trimmed_env(&read_env, "UID"),
            use_temp_dir_default_exec_root,
            skip_exec_root_probe: simulated_host
                || read_env("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_some()
                || bool_from_env(&read_env, "MOCK_CONTAINER", false),
            remote_cleanup_ttl: Duration::from_millis(duration_from_env(
                &read_env,
                "TAKD_REMOTE_CLEANUP_TTL_MS",
                DEFAULT_REMOTE_CLEANUP_TTL_MS,
            )),
            remote_cleanup_interval: Duration::from_millis(duration_from_env(
                &read_env,
                "TAKD_REMOTE_CLEANUP_INTERVAL_MS",
                DEFAULT_REMOTE_CLEANUP_INTERVAL_MS,
            )),
            remote_client_stale_ttl: Duration::from_millis(duration_from_env(
                &read_env,
                "TAKD_REMOTE_CLIENT_STALE_TTL_MS",
                DEFAULT_REMOTE_CLIENT_STALE_TTL_MS,
            )),
            remote_client_watchdog_interval: Duration::from_millis(duration_from_env(
                &read_env,
                "TAKD_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS",
                DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS,
            )),
            memory_pressure: MemoryPressureSettings::from_environment(&read_env),
            admission_oversubscribe_x: u64_from_env(
                &read_env,
                "TAKD_ADMISSION_OVERSUBSCRIBE_X",
                DEFAULT_ADMISSION_OVERSUBSCRIBE_X,
            )
            .max(1),
            default_container_cpu_cores: f64_from_env(
                &read_env,
                "TAKD_DEFAULT_CONTAINER_CPU_CORES",
                DEFAULT_CONTAINER_CPU_CORES,
            ),
            default_container_memory_mb: u64_from_env(
                &read_env,
                "TAKD_DEFAULT_CONTAINER_MEMORY_MB",
                DEFAULT_CONTAINER_MEMORY_MB,
            ),
            memory_pressure_enabled: bool_from_env(&read_env, "TAKD_MEMORY_PRESSURE_ENABLED", true),
            resource_sample_interval: Duration::from_millis(duration_from_env(
                &read_env,
                "TAK_TEST_RESOURCE_SAMPLE_MS",
                DEFAULT_RESOURCE_SAMPLE_INTERVAL_MS,
            )),
            host_baseline_sample_duration: Duration::from_millis(duration_from_env(
                &read_env,
                "TAKD_HOST_BASELINE_SAMPLE_MS",
                host_baseline_default_ms,
            )),
            ignore_host_usage_for_tests: bool_from_env(
                &read_env,
                "TAK_TEST_IGNORE_HOST_USAGE",
                simulated_host,
            ),
            test_memory_signal_path: optional_trimmed_env(&read_env, "TAK_TEST_MEMORY_SIGNAL_PATH")
                .map(PathBuf::from),
        }
    }
}
