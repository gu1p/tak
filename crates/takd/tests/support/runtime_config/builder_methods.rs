use super::*;

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

    pub fn with_default_container_resources(mut self, cpu_cores: f64, memory_mb: u64) -> Self {
        self.default_container_cpu_cores = Some(cpu_cores);
        self.default_container_memory_mb = Some(memory_mb);
        self
    }

    pub fn with_test_memory_signal(
        mut self,
        path: impl Into<PathBuf>,
        sample_interval: Duration,
    ) -> Self {
        self.test_memory_signal_path = Some(path.into());
        self.test_resource_sample_interval = Some(sample_interval);
        self
    }

    pub fn with_real_host_usage(mut self) -> Self {
        self.ignore_host_usage_for_tests = false;
        self
    }

    pub fn build(self) -> RemoteRuntimeConfig {
        environment::build(self)
    }
}
