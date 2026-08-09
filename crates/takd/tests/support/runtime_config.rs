use std::path::PathBuf;
use std::time::Duration;

use takd::RemoteRuntimeConfig;

mod environment;

#[derive(Default)]
pub struct RuntimeConfigBuilder {
    pub(super) explicit_remote_exec_root: Option<PathBuf>,
    pub(super) temp_dir: Option<PathBuf>,
    pub(super) docker_host: Option<String>,
    pub(super) skip_exec_root_probe: bool,
    pub(super) remote_cleanup_ttl: Option<Duration>,
    pub(super) remote_cleanup_interval: Option<Duration>,
    pub(super) remote_client_stale_ttl: Option<Duration>,
    pub(super) remote_client_watchdog_interval: Option<Duration>,
}

impl RuntimeConfigBuilder {
    pub fn with_explicit_remote_exec_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_remote_exec_root = Some(path.into());
        self
    }

    pub fn with_temp_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.temp_dir = Some(path.into());
        self
    }

    pub fn with_docker_host(mut self, host: impl Into<String>) -> Self {
        self.docker_host = Some(host.into());
        self
    }

    pub fn with_skip_exec_root_probe(mut self, skip: bool) -> Self {
        self.skip_exec_root_probe = skip;
        self
    }

    pub fn with_remote_cleanup_ttl(mut self, ttl: Duration) -> Self {
        self.remote_cleanup_ttl = Some(ttl);
        self
    }

    pub fn with_remote_cleanup_interval(mut self, interval: Duration) -> Self {
        self.remote_cleanup_interval = Some(interval);
        self
    }

    pub fn with_remote_client_stale_ttl(mut self, ttl: Duration) -> Self {
        self.remote_client_stale_ttl = Some(ttl);
        self
    }

    pub fn with_remote_client_watchdog_interval(mut self, interval: Duration) -> Self {
        self.remote_client_watchdog_interval = Some(interval);
        self
    }

    pub fn build(self) -> RemoteRuntimeConfig {
        environment::build(self)
    }
}

pub fn builder() -> RuntimeConfigBuilder {
    RuntimeConfigBuilder::default()
}

pub fn isolated() -> RemoteRuntimeConfig {
    builder().build()
}
