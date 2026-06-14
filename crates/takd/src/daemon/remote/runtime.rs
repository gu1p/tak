use std::path::PathBuf;
use std::time::Duration;

mod config_accessors;
mod env_parse;
mod memory_pressure_settings;

use env_parse::{bool_from_env, duration_from_env, optional_trimmed_env, u64_from_env};
pub(crate) use memory_pressure_settings::MemoryPressureSettings;

const DEFAULT_REMOTE_CLEANUP_TTL_MS: u64 = 15 * 60 * 1000;
const DEFAULT_REMOTE_CLEANUP_INTERVAL_MS: u64 = 60 * 1000;
const DEFAULT_REMOTE_CLIENT_STALE_TTL_MS: u64 = 600 * 1000;
const DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS: u64 = 1000;
const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";

const DEFAULT_ADMISSION_OVERSUBSCRIBE_X: u64 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    memory_pressure_enabled: bool,
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
            use_temp_dir_default_exec_root: false,
            skip_exec_root_probe: std::env::var("TAK_TEST_HOST_PLATFORM").is_ok()
                || std::env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_ok()
                || tak_core::mock::mock_container_enabled(),
            remote_cleanup_ttl: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLEANUP_TTL_MS",
                DEFAULT_REMOTE_CLEANUP_TTL_MS,
            )),
            remote_cleanup_interval: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLEANUP_INTERVAL_MS",
                DEFAULT_REMOTE_CLEANUP_INTERVAL_MS,
            )),
            remote_client_stale_ttl: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLIENT_STALE_TTL_MS",
                DEFAULT_REMOTE_CLIENT_STALE_TTL_MS,
            )),
            remote_client_watchdog_interval: Duration::from_millis(duration_from_env(
                "TAKD_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS",
                DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS,
            )),
            memory_pressure: MemoryPressureSettings::from_env(),
            admission_oversubscribe_x: u64_from_env(
                "TAKD_ADMISSION_OVERSUBSCRIBE_X",
                DEFAULT_ADMISSION_OVERSUBSCRIBE_X,
            )
            .max(1),
            memory_pressure_enabled: bool_from_env("TAKD_MEMORY_PRESSURE_ENABLED", true),
        }
    }

    pub fn for_tests() -> Self {
        Self {
            explicit_remote_exec_root: None,
            temp_dir: std::env::temp_dir(),
            // Tests that need a fake daemon override this via `with_docker_host`.
            // The default dead socket keeps janitors from reading process-global
            // `DOCKER_HOST` and crossing into another parallel test's daemon.
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
            // Strict (1x) in tests so the queue/cancel admission contract is
            // preserved; production over-admits via `from_env`. Tolerant behavior
            // is covered by dedicated admission unit tests.
            admission_oversubscribe_x: 1,
            // Off in tests: the controller reads real host memory and must never
            // spuriously pause/hold on a loaded CI machine. Production turns it on.
            memory_pressure_enabled: false,
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

    pub fn with_remote_client_stale_ttl(mut self, ttl: Duration) -> Self {
        self.remote_client_stale_ttl = ttl;
        self
    }

    pub fn with_remote_client_watchdog_interval(mut self, interval: Duration) -> Self {
        self.remote_client_watchdog_interval = interval;
        self
    }
}
