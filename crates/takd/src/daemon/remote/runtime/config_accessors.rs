//! Read-only accessors and derived execution-root paths for `RemoteRuntimeConfig`.

use std::path::PathBuf;
use std::time::Duration;

use super::{MemoryPressureSettings, REMOTE_EXEC_ROOT_DIR, RemoteRuntimeConfig};

impl RemoteRuntimeConfig {
    pub(crate) fn docker_host(&self) -> Option<&str> {
        self.docker_host.as_deref()
    }

    pub(crate) fn remote_cleanup_ttl(&self) -> Duration {
        self.remote_cleanup_ttl
    }

    pub(crate) fn remote_cleanup_interval(&self) -> Duration {
        self.remote_cleanup_interval
    }

    pub(crate) fn worker_cache_budget_bytes(&self) -> u64 {
        self.worker_cache_budget_bytes
    }

    pub(crate) fn memory_pressure(&self) -> MemoryPressureSettings {
        self.memory_pressure
    }

    pub(crate) fn admission_oversubscribe_x(&self) -> u64 {
        self.admission_oversubscribe_x
    }

    pub(crate) fn default_container_cpu_cores(&self) -> f64 {
        self.default_container_cpu_cores
    }

    pub(crate) fn default_container_memory_mb(&self) -> u64 {
        self.default_container_memory_mb
    }

    pub(crate) fn memory_pressure_enabled(&self) -> bool {
        self.memory_pressure_enabled
    }

    pub(crate) fn resource_sample_interval(&self) -> Duration {
        self.resource_sample_interval
    }

    pub(crate) fn host_baseline_sample_duration(&self) -> Duration {
        self.host_baseline_sample_duration
    }

    pub(crate) fn ignore_host_usage_for_tests(&self) -> bool {
        self.ignore_host_usage_for_tests
    }

    pub(crate) fn test_memory_signal_path(&self) -> Option<&PathBuf> {
        self.test_memory_signal_path.as_ref()
    }

    pub(crate) fn default_remote_execution_root_base(&self) -> PathBuf {
        if cfg!(unix) && !self.use_temp_dir_default_exec_root {
            return PathBuf::from("/var/tmp").join(REMOTE_EXEC_ROOT_DIR);
        }
        self.temp_dir.join(REMOTE_EXEC_ROOT_DIR)
    }

    pub(crate) fn initial_execution_root_base(&self) -> PathBuf {
        self.explicit_remote_exec_root
            .clone()
            .unwrap_or_else(|| self.default_remote_execution_root_base())
    }
}
