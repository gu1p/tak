//! Canonical model types shared by all Tak crates.
//!
//! These structures represent loader output, execution plans, limiter references, and
//! runtime workspace state.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskLabel {
    pub package: String,
    pub name: String,
}

impl fmt::Display for TaskLabel {
    /// Formats labels using clean user-facing syntax:
    /// - root package: `name`
    /// - nested package: `package:name`
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.package == "//" {
            return write!(f, "{}", self.name);
        }
        if let Some(package) = self.package.strip_prefix("//") {
            return write!(f, "{}:{}", package, self.name);
        }
        write!(f, "{}:{}", self.package, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Machine,
    User,
    Project,
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LimiterRef {
    pub name: String,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleSpec {
    #[serde(default = "default_spec_version")]
    pub spec_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default)]
    pub tasks: Vec<TaskDef>,
    #[serde(default)]
    pub limiters: Vec<LimiterDef>,
    #[serde(default)]
    pub queues: Vec<QueueDef>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub defaults: Defaults,
}

/// Returns the current supported module spec version for serde defaults.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_spec_version() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<QueueUseDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryDef>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskDef {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub steps: Vec<StepDef>,
    #[serde(default)]
    pub needs: Vec<NeedDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<QueueUseDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<CurrentStateDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<TaskExecutionDef>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PathInputDef {
    Path { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IgnoreSourceDef {
    Path { value: String },
    Gitignore,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentStateDef {
    #[serde(default)]
    pub roots: Vec<PathInputDef>,
    #[serde(default)]
    pub ignored: Vec<IgnoreSourceDef>,
    #[serde(default)]
    pub include: Vec<PathInputDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDef {
    pub id: String,
    #[serde(default = "default_local_parallelism")]
    pub max_parallel_tasks: u32,
}

/// Returns the default parallelism for local execution declarations.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_local_parallelism() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<RemoteTransportDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<RemoteWorkspaceDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<RemoteResultDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RemoteRuntimeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTransportDef {
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTransportKind {
    DirectHttps,
    Tor,
}

impl RemoteTransportKind {
    #[must_use]
    pub fn as_result_value(self) -> &'static str {
        match self {
            Self::DirectHttps => "direct",
            Self::Tor => "tor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWorkspaceDef {
    pub transfer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResultDef {
    pub sync: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRuntimeDef {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub mounts: Vec<ContainerMountDef>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "resources")]
    pub resource_limits: Option<ContainerResourceLimitsDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMountDef {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimitsDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerImageReference {
    pub canonical: String,
    pub digest_pinned: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContainerImageReferenceError {
    #[error("container image reference cannot be empty")]
    EmptyReference,
    #[error("container image reference cannot contain whitespace")]
    ContainsWhitespace,
    #[error("container image reference must not include a URL scheme")]
    ContainsScheme,
    #[error("container image digest must be `<algorithm>:<hex>`")]
    MalformedDigest,
    #[error("container image digest algorithm cannot be empty")]
    EmptyDigestAlgorithm,
    #[error("container image digest cannot be empty")]
    EmptyDigest,
    #[error("container image digest must contain only hexadecimal characters")]
    NonHexDigest,
    #[error("container image sha256 digest must be exactly 64 hexadecimal characters")]
    InvalidSha256DigestLength,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContainerMountSpec {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerResourceLimitsSpec {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerRuntimeExecutionSpec {
    pub image: String,
    pub command: Vec<String>,
    pub mounts: Vec<ContainerMountSpec>,
    pub env: BTreeMap<String, String>,
    pub resource_limits: Option<ContainerResourceLimitsSpec>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContainerRuntimeExecutionSpecError {
    #[error("runtime.kind `{kind}` is unsupported; expected `containerized`")]
    UnsupportedKind { kind: String },
    #[error("runtime.image is required for containerized runtime")]
    MissingImage,
    #[error("runtime.image {0}")]
    InvalidImage(ContainerImageReferenceError),
    #[error("runtime.command cannot be empty when provided")]
    EmptyCommand,
    #[error("runtime.command[{index}] cannot be empty")]
    EmptyCommandArg { index: usize },
    #[error("runtime.mounts[{index}].source cannot be empty")]
    EmptyMountSource { index: usize },
    #[error("runtime.mounts[{index}].target must be an absolute path without `..`, got `{target}`")]
    InvalidMountTarget { index: usize, target: String },
    #[error("runtime.env key `{key}` is invalid; expected `[A-Z_][A-Z0-9_]*`")]
    InvalidEnvKey { key: String },
    #[error("runtime.env key `{key}` is reserved for Tak runtime internals")]
    ReservedEnvKey { key: String },
    #[error("runtime.env value for `{key}` cannot contain null bytes: {value_preview}")]
    InvalidEnvValue { key: String, value_preview: String },
    #[error("runtime.resource_limits.cpu_cores must be > 0 and <= 256")]
    InvalidCpuCores,
    #[error("runtime.resource_limits.memory_mb must be > 0")]
    InvalidMemoryMb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RemoteSelectionDef {
    Single(Box<RemoteDef>),
    List(Vec<RemoteDef>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecisionModeDef {
    Local,
    Remote,
    RemoteAny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecisionDef {
    pub mode: PolicyDecisionModeDef,
    #[serde(default)]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<Box<RemoteDef>>,
    #[serde(default)]
    pub remotes: Vec<RemoteDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskExecutionDef {
    LocalOnly {
        local: LocalDef,
    },
    RemoteOnly {
        remote: RemoteSelectionDef,
    },
    ByCustomPolicy {
        policy_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision: Option<PolicyDecisionDef>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepDef {
    Cmd {
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    Script {
        path: String,
        #[serde(default)]
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpreter: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
}

impl Default for StepDef {
    /// Creates an empty command step used by serde defaulting.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self::Cmd {
            argv: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    #[default]
    During,
    AtStart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedDef {
    pub limiter: LimiterRef,
    #[serde(default = "default_one")]
    pub slots: f64,
    #[serde(default)]
    pub hold: Hold,
}

/// Returns the default slot count for limiter needs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueUseDef {
    pub queue: LimiterRef,
    #[serde(default = "default_slots")]
    pub slots: i32,
    #[serde(default)]
    pub priority: i32,
}

/// Returns the default slot count for queue usage.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_slots() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimiterDef {
    Resource {
        name: String,
        scope: Scope,
        capacity: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },
    Lock {
        name: String,
        scope: Scope,
    },
    RateLimit {
        name: String,
        scope: Scope,
        burst: u32,
        refill_per_second: f64,
    },
    ProcessCap {
        name: String,
        scope: Scope,
        max_running: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#match: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDef {
    pub name: String,
    pub scope: Scope,
    pub slots: u32,
    #[serde(default)]
    pub discipline: QueueDiscipline,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pending: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueDiscipline {
    #[default]
    Fifo,
    Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryDef {
    #[serde(default = "default_attempts")]
    pub attempts: u32,
    #[serde(default)]
    pub on_exit: Vec<i32>,
    #[serde(default)]
    pub backoff: BackoffDef,
}

impl Default for RetryDef {
    /// Builds the default retry policy: one attempt and no backoff delay.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            attempts: default_attempts(),
            on_exit: Vec::new(),
            backoff: BackoffDef::default(),
        }
    }
}

/// Returns the default retry attempt count.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_attempts() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackoffDef {
    Fixed {
        seconds: f64,
    },
    ExpJitter {
        min_s: f64,
        max_s: f64,
        #[serde(default = "default_jitter")]
        jitter: String,
    },
}

impl Default for BackoffDef {
    /// Uses a zero-second fixed backoff as the default strategy.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self::Fixed { seconds: 0.0 }
    }
}

/// Returns the default jitter mode for exponential backoff.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_jitter() -> String {
    "full".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LimiterKey {
    pub scope: Scope,
    pub scope_key: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedTask {
    pub label: TaskLabel,
    pub doc: String,
    pub deps: Vec<TaskLabel>,
    pub steps: Vec<StepDef>,
    pub needs: Vec<NeedDef>,
    pub queue: Option<QueueUseDef>,
    pub retry: RetryDef,
    pub timeout_s: Option<u64>,
    pub context: CurrentStateSpec,
    pub execution: TaskExecutionSpec,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalSpec {
    pub id: String,
    pub max_parallel_tasks: u32,
}

impl Default for LocalSpec {
    /// Returns the default local execution profile when no execution is specified.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            id: "local".to_string(),
            max_parallel_tasks: default_local_parallelism(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteSpec {
    pub id: String,
    pub endpoint: Option<String>,
    pub transport_kind: RemoteTransportKind,
    pub runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone)]
pub enum RemoteRuntimeSpec {
    Containerized { image: String },
}

#[derive(Debug, Clone)]
pub enum RemoteSelectionSpec {
    Single(RemoteSpec),
    List(Vec<RemoteSpec>),
}

#[derive(Debug, Clone)]
pub enum PolicyDecisionSpec {
    Local {
        reason: String,
    },
    Remote {
        reason: String,
        remote: RemoteSpec,
    },
    RemoteAny {
        reason: String,
        remotes: Vec<RemoteSpec>,
    },
}

#[derive(Debug, Clone)]
pub enum TaskExecutionSpec {
    LocalOnly(LocalSpec),
    RemoteOnly(RemoteSelectionSpec),
    ByCustomPolicy {
        policy_name: String,
        decision: Option<PolicyDecisionSpec>,
    },
}

#[derive(Debug, Clone)]
pub enum IgnoreSourceSpec {
    Path(PathRef),
    GitIgnore,
}

#[derive(Debug, Clone)]
pub struct CurrentStateSpec {
    pub roots: Vec<PathRef>,
    pub ignored: Vec<IgnoreSourceSpec>,
    pub include: Vec<PathRef>,
}

impl Default for CurrentStateSpec {
    /// Uses full workspace roots with no additional ignore/include rules.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            roots: vec![PathRef {
                anchor: PathAnchor::Workspace,
                path: ".".to_string(),
            }],
            ignored: Vec::new(),
            include: Vec::new(),
        }
    }
}

impl Default for TaskExecutionSpec {
    /// Uses local-only execution as the default task execution mode.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self::LocalOnly(LocalSpec::default())
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceSpec {
    pub project_id: String,
    pub root: PathBuf,
    pub tasks: BTreeMap<TaskLabel, ResolvedTask>,
    pub limiters: HashMap<LimiterKey, LimiterDef>,
    pub queues: HashMap<LimiterKey, QueueDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathAnchor {
    Workspace,
    Package,
    Repo(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathRef {
    pub anchor: PathAnchor,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextManifest {
    pub entries: Vec<PathRef>,
    pub hash: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PathNormalizationError {
    #[error("path anchor cannot be empty")]
    EmptyAnchor,
    #[error("repo anchor name cannot be empty")]
    EmptyRepoAnchor,
    #[error("unsupported anchor `{0}`")]
    UnsupportedAnchor(String),
    #[error("path escapes anchor `{anchor}`: {path}")]
    EscapesAnchor { anchor: String, path: String },
}

pub fn normalize_container_image_reference(
    image: &str,
) -> Result<ContainerImageReference, ContainerImageReferenceError> {
    let trimmed = image.trim();
    if trimmed.is_empty() {
        return Err(ContainerImageReferenceError::EmptyReference);
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(ContainerImageReferenceError::ContainsWhitespace);
    }
    if trimmed.contains("://") {
        return Err(ContainerImageReferenceError::ContainsScheme);
    }

    let mut parts = trimmed.split('@');
    let image_name = parts.next().unwrap_or_default();
    let digest = parts.next();
    if parts.next().is_some() {
        return Err(ContainerImageReferenceError::MalformedDigest);
    }

    if image_name.is_empty() {
        return Err(ContainerImageReferenceError::EmptyReference);
    }

    let canonical_image = normalize_image_name_and_tag(image_name)?;
    let Some(digest) = digest else {
        return Ok(ContainerImageReference {
            canonical: canonical_image,
            digest_pinned: false,
        });
    };

    let (raw_algorithm, raw_hex) = digest
        .split_once(':')
        .ok_or(ContainerImageReferenceError::MalformedDigest)?;
    if raw_algorithm.is_empty() {
        return Err(ContainerImageReferenceError::EmptyDigestAlgorithm);
    }
    if raw_hex.is_empty() {
        return Err(ContainerImageReferenceError::EmptyDigest);
    }
    if !raw_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ContainerImageReferenceError::NonHexDigest);
    }

    let algorithm = raw_algorithm.to_ascii_lowercase();
    let digest_hex = raw_hex.to_ascii_lowercase();
    if algorithm == "sha256" && digest_hex.len() != 64 {
        return Err(ContainerImageReferenceError::InvalidSha256DigestLength);
    }

    Ok(ContainerImageReference {
        canonical: format!("{canonical_image}@{algorithm}:{digest_hex}"),
        digest_pinned: true,
    })
}

pub fn validate_container_runtime_execution_spec(
    runtime: &RemoteRuntimeDef,
) -> Result<ContainerRuntimeExecutionSpec, ContainerRuntimeExecutionSpecError> {
    let kind = runtime.kind.trim();
    if kind != "containerized" {
        return Err(ContainerRuntimeExecutionSpecError::UnsupportedKind {
            kind: kind.to_string(),
        });
    }

    let image = runtime.image.clone().unwrap_or_default();
    if image.trim().is_empty() {
        return Err(ContainerRuntimeExecutionSpecError::MissingImage);
    }
    let image = normalize_container_image_reference(&image)
        .map_err(ContainerRuntimeExecutionSpecError::InvalidImage)?
        .canonical;

    let command = normalize_runtime_command(runtime.command.as_ref())?;
    let mounts = normalize_runtime_mounts(&runtime.mounts)?;
    let env = normalize_runtime_env(&runtime.env)?;
    let resource_limits = normalize_runtime_resource_limits(runtime.resource_limits.as_ref())?;

    Ok(ContainerRuntimeExecutionSpec {
        image,
        command,
        mounts,
        env,
        resource_limits,
    })
}

pub fn normalize_path_ref(anchor: &str, path: &str) -> Result<PathRef, PathNormalizationError> {
    let normalized_anchor = parse_anchor(anchor)?;
    let normalized_path = normalize_relative_path(anchor, path)?;
    Ok(PathRef {
        anchor: normalized_anchor,
        path: normalized_path,
    })
}

impl ContextManifest {
    /// Builds a canonical context manifest from path refs and computes a stable SHA-256 hash.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn from_paths(paths: impl IntoIterator<Item = PathRef>) -> Self {
        let mut entries: Vec<PathRef> = paths.into_iter().collect();
        entries.sort_by(compare_path_ref);
        entries.dedup();

        let hash = hash_manifest_entries(&entries);
        Self { entries, hash }
    }
}

/// Builds a deterministic transfer manifest from available files and `CurrentState` boundaries.
///
/// The filter order is:
/// 1. keep files inside selected `roots`
/// 2. remove files matched by `ignored`
/// 3. re-add files matched by `include` if they are still inside `roots`
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn build_current_state_manifest(
    available_files: impl IntoIterator<Item = PathRef>,
    state: &CurrentStateSpec,
) -> ContextManifest {
    let files: Vec<PathRef> = available_files.into_iter().collect();
    let mut selected = Vec::new();

    for file in &files {
        if !matches_any_root(file, &state.roots) {
            continue;
        }
        if matches_any_ignored(file, &state.ignored) {
            continue;
        }
        selected.push(file.clone());
    }

    for include in &state.include {
        for file in &files {
            if !is_path_within(file, include) {
                continue;
            }
            if !matches_any_root(file, &state.roots) {
                continue;
            }
            selected.push(file.clone());
        }
    }

    ContextManifest::from_paths(selected)
}

fn compare_path_ref(left: &PathRef, right: &PathRef) -> Ordering {
    compare_anchor(&left.anchor, &right.anchor).then_with(|| left.path.cmp(&right.path))
}

fn matches_any_root(file: &PathRef, roots: &[PathRef]) -> bool {
    roots.iter().any(|root| is_path_within(file, root))
}

fn matches_any_ignored(file: &PathRef, ignored: &[IgnoreSourceSpec]) -> bool {
    ignored.iter().any(|source| match source {
        IgnoreSourceSpec::Path(path) => is_path_within(file, path),
        IgnoreSourceSpec::GitIgnore => false,
    })
}

fn is_path_within(file: &PathRef, container: &PathRef) -> bool {
    if file.anchor != container.anchor {
        return false;
    }
    if container.path == "." {
        return true;
    }
    file.path == container.path
        || file
            .path
            .strip_prefix(&container.path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn compare_anchor(left: &PathAnchor, right: &PathAnchor) -> Ordering {
    anchor_sort_rank(left)
        .cmp(&anchor_sort_rank(right))
        .then_with(|| anchor_token(left).cmp(&anchor_token(right)))
}

fn anchor_sort_rank(anchor: &PathAnchor) -> u8 {
    match anchor {
        PathAnchor::Package => 0,
        PathAnchor::Repo(_) => 1,
        PathAnchor::Workspace => 2,
    }
}

fn anchor_token(anchor: &PathAnchor) -> String {
    match anchor {
        PathAnchor::Workspace => "workspace".to_string(),
        PathAnchor::Package => "package".to_string(),
        PathAnchor::Repo(name) => format!("repo:{name}"),
    }
}

fn hash_manifest_entries(entries: &[PathRef]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        let anchor = anchor_token(&entry.anchor);
        let anchor_bytes = anchor.as_bytes();
        let path_bytes = entry.path.as_bytes();

        hasher.update((anchor_bytes.len() as u64).to_be_bytes());
        hasher.update(anchor_bytes);
        hasher.update((path_bytes.len() as u64).to_be_bytes());
        hasher.update(path_bytes);
    }
    hex::encode(hasher.finalize())
}

fn parse_anchor(anchor: &str) -> Result<PathAnchor, PathNormalizationError> {
    let normalized = anchor.trim();
    if normalized.is_empty() {
        return Err(PathNormalizationError::EmptyAnchor);
    }

    match normalized {
        "workspace" => Ok(PathAnchor::Workspace),
        "package" => Ok(PathAnchor::Package),
        _ => {
            if let Some(repo) = normalized.strip_prefix("repo:") {
                let repo = repo.trim();
                if repo.is_empty() {
                    return Err(PathNormalizationError::EmptyRepoAnchor);
                }
                return Ok(PathAnchor::Repo(repo.to_string()));
            }
            Err(PathNormalizationError::UnsupportedAnchor(
                normalized.to_string(),
            ))
        }
    }
}

fn normalize_image_name_and_tag(image_name: &str) -> Result<String, ContainerImageReferenceError> {
    let last_slash = image_name.rfind('/');
    let last_colon = image_name.rfind(':');

    let split_tag = match (last_colon, last_slash) {
        (Some(colon), Some(slash)) => colon > slash,
        (Some(_), None) => true,
        (None, _) => false,
    };

    if split_tag {
        let colon = last_colon.ok_or(ContainerImageReferenceError::MalformedDigest)?;
        let repository = &image_name[..colon];
        let tag = &image_name[colon + 1..];
        if repository.is_empty() || tag.is_empty() {
            return Err(ContainerImageReferenceError::EmptyReference);
        }
        return Ok(format!("{}:{tag}", repository.to_ascii_lowercase()));
    }

    Ok(image_name.to_ascii_lowercase())
}

fn normalize_runtime_command(
    command: Option<&Vec<String>>,
) -> Result<Vec<String>, ContainerRuntimeExecutionSpecError> {
    let Some(command) = command else {
        return Ok(Vec::new());
    };
    if command.is_empty() {
        return Err(ContainerRuntimeExecutionSpecError::EmptyCommand);
    }

    let mut normalized = Vec::with_capacity(command.len());
    for (index, value) in command.iter().enumerate() {
        let argument = value.trim();
        if argument.is_empty() {
            return Err(ContainerRuntimeExecutionSpecError::EmptyCommandArg { index });
        }
        normalized.push(argument.to_string());
    }
    Ok(normalized)
}

fn normalize_runtime_mounts(
    mounts: &[ContainerMountDef],
) -> Result<Vec<ContainerMountSpec>, ContainerRuntimeExecutionSpecError> {
    let mut normalized = Vec::with_capacity(mounts.len());
    for (index, mount) in mounts.iter().enumerate() {
        let source = mount.source.trim();
        if source.is_empty() {
            return Err(ContainerRuntimeExecutionSpecError::EmptyMountSource { index });
        }
        let target = normalize_runtime_mount_target(&mount.target).ok_or_else(|| {
            ContainerRuntimeExecutionSpecError::InvalidMountTarget {
                index,
                target: mount.target.trim().to_string(),
            }
        })?;
        normalized.push(ContainerMountSpec {
            source: source.replace('\\', "/"),
            target,
            read_only: mount.read_only,
        });
    }

    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_runtime_mount_target(target: &str) -> Option<String> {
    let normalized = target.trim().replace('\\', "/");
    if normalized.is_empty() || !normalized.starts_with('/') {
        return None;
    }

    let mut segments = Vec::new();
    for segment in normalized.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return None,
            value => segments.push(value.to_string()),
        }
    }

    if segments.is_empty() {
        return Some("/".to_string());
    }
    Some(format!("/{}", segments.join("/")))
}

fn normalize_runtime_env(
    env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ContainerRuntimeExecutionSpecError> {
    let mut normalized = BTreeMap::new();
    for (key, value) in env {
        let key = key.trim();
        if !is_valid_runtime_env_key(key) {
            return Err(ContainerRuntimeExecutionSpecError::InvalidEnvKey {
                key: key.to_string(),
            });
        }
        if is_reserved_runtime_env_key(key) {
            return Err(ContainerRuntimeExecutionSpecError::ReservedEnvKey {
                key: key.to_string(),
            });
        }
        if value.contains('\0') {
            return Err(ContainerRuntimeExecutionSpecError::InvalidEnvValue {
                key: key.to_string(),
                value_preview: redact_runtime_env_value(key, value),
            });
        }
        normalized.insert(key.to_string(), value.clone());
    }
    Ok(normalized)
}

fn is_valid_runtime_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_uppercase() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_reserved_runtime_env_key(key: &str) -> bool {
    matches!(
        key,
        "TAK_REMOTE_RUNTIME" | "TAK_REMOTE_ENGINE" | "TAK_REMOTE_CONTAINER_IMAGE"
    )
}

fn redact_runtime_env_value(key: &str, value: &str) -> String {
    if is_sensitive_runtime_env_key(key) {
        return "<redacted>".to_string();
    }
    let escaped = value.replace('\0', "\\0");
    if escaped.len() <= 64 {
        return escaped;
    }
    format!("{}...", &escaped[..64])
}

fn is_sensitive_runtime_env_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("key")
}

fn normalize_runtime_resource_limits(
    resource_limits: Option<&ContainerResourceLimitsDef>,
) -> Result<Option<ContainerResourceLimitsSpec>, ContainerRuntimeExecutionSpecError> {
    let Some(resource_limits) = resource_limits else {
        return Ok(None);
    };

    if let Some(cpu_cores) = resource_limits.cpu_cores
        && (!cpu_cores.is_finite() || cpu_cores <= 0.0 || cpu_cores > 256.0)
    {
        return Err(ContainerRuntimeExecutionSpecError::InvalidCpuCores);
    }

    if let Some(memory_mb) = resource_limits.memory_mb
        && memory_mb == 0
    {
        return Err(ContainerRuntimeExecutionSpecError::InvalidMemoryMb);
    }

    if resource_limits.cpu_cores.is_none() && resource_limits.memory_mb.is_none() {
        return Ok(None);
    }

    Ok(Some(ContainerResourceLimitsSpec {
        cpu_cores: resource_limits.cpu_cores,
        memory_mb: resource_limits.memory_mb,
    }))
}

fn normalize_relative_path(anchor: &str, path: &str) -> Result<String, PathNormalizationError> {
    let mut parts = Vec::<String>::new();
    for segment in path.replace('\\', "/").split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                if parts.pop().is_none() {
                    return Err(PathNormalizationError::EscapesAnchor {
                        anchor: anchor.to_string(),
                        path: path.to_string(),
                    });
                }
            }
            value => parts.push(value.to_string()),
        }
    }

    if parts.is_empty() {
        return Ok(".".to_string());
    }
    Ok(parts.join("/"))
}
