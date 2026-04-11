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
    pub container_runtime: Option<RemoteRuntimeSpec>,
    pub execution: TaskExecutionSpec,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalSpec {
    pub id: String,
    pub max_parallel_tasks: u32,
    pub runtime: Option<RemoteRuntimeSpec>,
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
            runtime: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteSpec {
    pub pool: Option<String>,
    pub required_tags: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub transport_kind: RemoteTransportKind,
    pub runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRuntimeSourceSpec {
    Image { image: String },
    Dockerfile { dockerfile: PathRef, build_context: PathRef },
}

#[derive(Debug, Clone)]
pub enum RemoteRuntimeSpec {
    Containerized { source: ContainerRuntimeSourceSpec },
}

#[derive(Debug, Clone)]
pub enum PolicyDecisionSpec {
    Local {
        reason: String,
        local: Option<LocalSpec>,
    },
    Remote {
        reason: String,
        remote: RemoteSpec,
    },
}

#[derive(Debug, Clone)]
pub enum TaskExecutionSpec {
    LocalOnly(LocalSpec),
    RemoteOnly(RemoteSpec),
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
