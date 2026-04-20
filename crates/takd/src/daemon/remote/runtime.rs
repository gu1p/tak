use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_REMOTE_CLEANUP_TTL_MS: u64 = 15 * 60 * 1000;
const DEFAULT_REMOTE_CLEANUP_INTERVAL_MS: u64 = 60 * 1000;
const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRuntimeConfig {
    explicit_remote_exec_root: Option<PathBuf>,
    temp_dir: PathBuf,
    docker_host: Option<String>,
    podman_socket: Option<String>,
    runtime_dir: Option<String>,
    uid: Option<String>,
    skip_exec_root_probe: bool,
    remote_cleanup_ttl: Duration,
    remote_cleanup_interval: Duration,
}

impl RemoteRuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            explicit_remote_exec_root: optional_trimmed_env("TAKD_REMOTE_EXEC_ROOT")
                .map(PathBuf::from),
            temp_dir: std::env::temp_dir(),
            docker_host: optional_trimmed_env("DOCKER_HOST"),
            podman_socket: optional_trimmed_env("TAK_PODMAN_SOCKET"),
            runtime_dir: optional_trimmed_env("XDG_RUNTIME_DIR"),
            uid: optional_trimmed_env("UID"),
            skip_exec_root_probe: std::env::var("TAK_TEST_HOST_PLATFORM").is_ok()
                || std::env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_ok(),
            remote_cleanup_ttl: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLEANUP_TTL_MS",
                DEFAULT_REMOTE_CLEANUP_TTL_MS,
            )),
            remote_cleanup_interval: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLEANUP_INTERVAL_MS",
                DEFAULT_REMOTE_CLEANUP_INTERVAL_MS,
            )),
        }
    }

    pub fn for_tests() -> Self {
        Self {
            explicit_remote_exec_root: None,
            temp_dir: std::env::temp_dir(),
            docker_host: None,
            podman_socket: None,
            runtime_dir: None,
            uid: None,
            skip_exec_root_probe: false,
            remote_cleanup_ttl: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_TTL_MS),
            remote_cleanup_interval: Duration::from_millis(DEFAULT_REMOTE_CLEANUP_INTERVAL_MS),
        }
    }

    pub fn with_explicit_remote_exec_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_remote_exec_root = Some(path.into());
        self
    }

    pub fn with_temp_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.temp_dir = path.into();
        self
    }

    pub fn with_docker_host(mut self, host: impl Into<String>) -> Self {
        self.docker_host = Some(host.into());
        self
    }

    pub fn with_podman_socket(mut self, socket: impl Into<String>) -> Self {
        self.podman_socket = Some(socket.into());
        self
    }

    pub fn with_runtime_dir(mut self, runtime_dir: impl Into<String>) -> Self {
        self.runtime_dir = Some(runtime_dir.into());
        self
    }

    pub fn with_uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = Some(uid.into());
        self
    }

    pub fn with_skip_exec_root_probe(mut self, skip: bool) -> Self {
        self.skip_exec_root_probe = skip;
        self
    }

    pub fn with_remote_cleanup_ttl(mut self, ttl: Duration) -> Self {
        self.remote_cleanup_ttl = ttl;
        self
    }

    pub fn with_remote_cleanup_interval(mut self, interval: Duration) -> Self {
        self.remote_cleanup_interval = interval;
        self
    }

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

    pub(crate) fn default_remote_execution_root_base(&self) -> PathBuf {
        if cfg!(unix) {
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

fn optional_trimmed_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn duration_from_env(name: &str, default_ms: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_ms)
}
