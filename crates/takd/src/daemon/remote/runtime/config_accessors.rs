//! Read-only accessors and derived execution-root paths for `RemoteRuntimeConfig`.

use std::path::PathBuf;
use std::time::Duration;

use super::{MemoryPressureSettings, REMOTE_EXEC_ROOT_DIR, RemoteRuntimeConfig};

impl RemoteRuntimeConfig {
    pub(crate) fn explicit_remote_exec_root(&self) -> Option<&PathBuf> {
        self.explicit_remote_exec_root.as_ref()
    }

    pub(crate) fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    pub(crate) fn docker_host(&self) -> Option<&str> {
        self.docker_host.as_deref()
    }

    pub(crate) fn podman_socket(&self) -> Option<&str> {
        self.podman_socket.as_deref()
    }

    pub(crate) fn runtime_dir(&self) -> Option<&str> {
        self.runtime_dir.as_deref()
    }

    pub(crate) fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    pub(crate) fn skip_exec_root_probe(&self) -> bool {
        self.skip_exec_root_probe
    }

    pub(crate) fn remote_cleanup_ttl(&self) -> Duration {
        self.remote_cleanup_ttl
    }

    pub(crate) fn remote_cleanup_interval(&self) -> Duration {
        self.remote_cleanup_interval
    }

    pub(crate) fn remote_client_stale_ttl(&self) -> Duration {
        self.remote_client_stale_ttl
    }

    pub(crate) fn remote_client_watchdog_interval(&self) -> Duration {
        self.remote_client_watchdog_interval
    }

    pub(crate) fn memory_pressure(&self) -> MemoryPressureSettings {
        self.memory_pressure
    }

    pub(crate) fn admission_oversubscribe_x(&self) -> u64 {
        self.admission_oversubscribe_x
    }

    pub(crate) fn memory_pressure_enabled(&self) -> bool {
        self.memory_pressure_enabled
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
