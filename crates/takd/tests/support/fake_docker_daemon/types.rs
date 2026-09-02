use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerOperation {
    Created(String),
    RemovalAttempted(String),
    Removed(String),
    UnpauseAttempted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRecord {
    pub container_id: String,
    pub image: Option<String>,
    pub cmd: Vec<String>,
    pub user: Option<String>,
    pub working_dir: Option<String>,
    pub binds: Vec<String>,
    pub bind_modes: BTreeMap<String, u32>,
    pub labels: BTreeMap<String, String>,
    pub env: Vec<String>,
    pub nano_cpus: Option<i64>,
    /// Engine state reported in the container list (`running`/`paused`).
    pub state: String,
}

impl CreateRecord {
    pub fn is_probe(&self) -> bool {
        self.cmd
            .iter()
            .any(|value| value.contains(".tak-mount-visible"))
    }

    pub fn bind_source(&self) -> Option<PathBuf> {
        self.binds
            .first()
            .and_then(|bind| bind.split(':').next())
            .map(PathBuf::from)
    }
}

#[derive(Debug, Clone)]
pub struct FakeDockerConfig {
    pub visible_roots: Vec<PathBuf>,
    pub image_present: bool,
    pub arch: String,
    pub version_fails: bool,
    pub wait_response_delay: Duration,
    pub ping_response_delay: Duration,
    pub memory_usage_bytes: u64,
    pub removal_failures: usize,
    pub oom_killed: bool,
}

impl Default for FakeDockerConfig {
    fn default() -> Self {
        Self {
            visible_roots: Vec::new(),
            image_present: true,
            arch: "x86_64".to_string(),
            version_fails: false,
            wait_response_delay: Duration::ZERO,
            ping_response_delay: Duration::ZERO,
            memory_usage_bytes: 0,
            removal_failures: 0,
            oom_killed: false,
        }
    }
}
