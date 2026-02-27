//! Canonical model types shared by all Tak crates.
//!
//! These structures represent loader output, execution plans, limiter references, and
//! runtime workspace state.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub tags: Vec<String>,
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
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSpec {
    pub project_id: String,
    pub root: PathBuf,
    pub tasks: BTreeMap<TaskLabel, ResolvedTask>,
    pub limiters: HashMap<LimiterKey, LimiterDef>,
    pub queues: HashMap<LimiterKey, QueueDef>,
}
