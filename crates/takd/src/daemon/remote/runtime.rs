use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_REMOTE_CLEANUP_TTL_MS: u64 = 15 * 60 * 1000;
const DEFAULT_REMOTE_CLEANUP_INTERVAL_MS: u64 = 60 * 1000;
const DEFAULT_REMOTE_CLIENT_STALE_TTL_MS: u64 = 600 * 1000;
const DEFAULT_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS: u64 = 1000;
const REMOTE_EXEC_ROOT_DIR: &str = "takd-remote-exec";

const DEFAULT_MEMORY_PRESSURE_INTERVAL_MS: u64 = 1000;
const DEFAULT_MEMORY_PRESSURE_PAUSE_PCT: u64 = 15;
const DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB: u64 = 2048;
const DEFAULT_MEMORY_PRESSURE_RESUME_PCT: u64 = 25;
const DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT: u64 = 7;
const DEFAULT_MEMORY_PRESSURE_MIN_RUNNING: usize = 1;
const DEFAULT_ADMISSION_OVERSUBSCRIBE_X: u64 = 16;

/// Tunables for the never-kill memory-pressure controller. Percentages are of
/// host total memory. `pause_floor_mb` is an absolute MemAvailable floor: the
/// controller pauses when available drops below the LARGER of `pause_pct`% of
/// total or this floor, so the threshold stays sane on both large nodes (where a
/// percentage is many GiB) and small ones (where it is too tight). Invariant
/// `emergency_pct < pause_pct < resume_pct` (the resume/pause gap is the
/// hysteresis dead-band); a misconfigured set falls back to defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryPressureSettings {
    pub(crate) interval: Duration,
    pub(crate) pause_pct: u64,
    pub(crate) pause_floor_mb: u64,
    pub(crate) resume_pct: u64,
    pub(crate) emergency_pct: u64,
    pub(crate) min_running: usize,
}

impl MemoryPressureSettings {
    pub(crate) fn defaults() -> Self {
        Self {
            interval: Duration::from_millis(DEFAULT_MEMORY_PRESSURE_INTERVAL_MS),
            pause_pct: DEFAULT_MEMORY_PRESSURE_PAUSE_PCT,
            pause_floor_mb: DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB,
            resume_pct: DEFAULT_MEMORY_PRESSURE_RESUME_PCT,
            emergency_pct: DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT,
            min_running: DEFAULT_MEMORY_PRESSURE_MIN_RUNNING,
        }
    }

    fn from_env() -> Self {
        Self {
            interval: Duration::from_millis(duration_from_env(
                "TAKD_MEMORY_PRESSURE_INTERVAL_MS",
                DEFAULT_MEMORY_PRESSURE_INTERVAL_MS,
            )),
            pause_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_PAUSE_PCT",
                DEFAULT_MEMORY_PRESSURE_PAUSE_PCT,
            ),
            pause_floor_mb: u64_from_env(
                "TAKD_MEMORY_PRESSURE_PAUSE_FLOOR_MB",
                DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB,
            ),
            resume_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_RESUME_PCT",
                DEFAULT_MEMORY_PRESSURE_RESUME_PCT,
            ),
            emergency_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_EMERGENCY_PCT",
                DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT,
            ),
            min_running: usize_from_env(
                "TAKD_MEMORY_PRESSURE_MIN_RUNNING",
                DEFAULT_MEMORY_PRESSURE_MIN_RUNNING,
            ),
        }
        .sanitized()
    }

    /// Keep the hysteresis band valid; on any ordering violation, reset the three
    /// watermarks to defaults while preserving interval/floor/min_running.
    fn sanitized(self) -> Self {
        if self.emergency_pct < self.pause_pct && self.pause_pct < self.resume_pct {
            return self;
        }
        Self {
            interval: self.interval,
            pause_floor_mb: self.pause_floor_mb,
            min_running: self.min_running,
            ..Self::defaults()
        }
    }
}

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

fn u64_from_env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn usize_from_env(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn bool_from_env(name: &str, default: bool) -> bool {
    match std::env::var(name)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("1" | "true" | "yes" | "on") => true,
        Some("0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

/// Parse a 1..=100 percentage; out-of-range or unparsable falls back to default.
fn percent_from_env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| (1..=100).contains(value))
        .unwrap_or(default)
}
