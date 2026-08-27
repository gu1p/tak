use std::path::PathBuf;
use std::time::Duration;

use takd::RemoteRuntimeConfig;

mod builder_methods;
mod environment;

pub struct RuntimeConfigBuilder {
    pub(super) explicit_remote_exec_root: Option<PathBuf>,
    pub(super) temp_dir: Option<PathBuf>,
    pub(super) docker_host: Option<String>,
    pub(super) skip_exec_root_probe: bool,
    pub(super) remote_cleanup_ttl: Option<Duration>,
    pub(super) remote_cleanup_interval: Option<Duration>,
    pub(super) remote_client_stale_ttl: Option<Duration>,
    pub(super) remote_client_watchdog_interval: Option<Duration>,
    pub(super) default_container_cpu_cores: Option<f64>,
    pub(super) default_container_memory_mb: Option<u64>,
    pub(super) test_memory_signal_path: Option<PathBuf>,
    pub(super) test_resource_sample_interval: Option<Duration>,
    pub(super) ignore_host_usage_for_tests: bool,
}

impl Default for RuntimeConfigBuilder {
    fn default() -> Self {
        Self {
            explicit_remote_exec_root: None,
            temp_dir: None,
            docker_host: None,
            skip_exec_root_probe: false,
            remote_cleanup_ttl: None,
            remote_cleanup_interval: None,
            remote_client_stale_ttl: None,
            remote_client_watchdog_interval: None,
            default_container_cpu_cores: None,
            default_container_memory_mb: None,
            test_memory_signal_path: None,
            test_resource_sample_interval: None,
            ignore_host_usage_for_tests: true,
        }
    }
}

pub fn builder() -> RuntimeConfigBuilder {
    RuntimeConfigBuilder::default()
}

pub fn isolated() -> RemoteRuntimeConfig {
    builder().build()
}
