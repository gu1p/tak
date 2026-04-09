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
    pub includes: Vec<PathInputDef>,
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
