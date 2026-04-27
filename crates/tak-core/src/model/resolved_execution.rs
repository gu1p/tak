use super::*;

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
    pub selection: RemoteSelectionSpec,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RemoteSelectionSpec {
    #[default]
    Sequential,
    Shuffle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRuntimeSourceSpec {
    Image {
        image: String,
    },
    Dockerfile {
        dockerfile: PathRef,
        build_context: PathRef,
    },
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
pub enum ExecutionPlacementSpec {
    Local(LocalSpec),
    Remote(RemoteSpec),
}

#[derive(Debug, Clone)]
pub struct ExecutionPolicySpec {
    pub name: String,
    pub placements: Vec<ExecutionPlacementSpec>,
    pub doc: String,
}

#[derive(Debug, Clone)]
pub enum TaskExecutionSpec {
    LocalOnly(LocalSpec),
    RemoteOnly(RemoteSpec),
    ByCustomPolicy {
        policy_name: String,
        decision: Option<PolicyDecisionSpec>,
    },
    ByExecutionPolicy {
        name: String,
        placements: Vec<ExecutionPlacementSpec>,
    },
    UseSession {
        name: String,
        cascade: bool,
    },
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
