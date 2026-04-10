#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredRemoteDiagnostic {
    pub pool: Option<String>,
    pub required_tags: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub transport_kind: RemoteTransportKind,
}

impl RequiredRemoteDiagnostic {
    fn from_spec(remote: &RemoteSpec) -> Self {
        Self {
            pool: remote.pool.clone(),
            required_tags: remote.required_tags.clone(),
            required_capabilities: remote.required_capabilities.clone(),
            transport_kind: remote.transport_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteCandidateRejection {
    PoolMismatch {
        required: String,
        available: Vec<String>,
    },
    MissingTags {
        missing: Vec<String>,
        available: Vec<String>,
    },
    MissingCapabilities {
        missing: Vec<String>,
        available: Vec<String>,
    },
    TransportMismatch {
        required: RemoteTransportKind,
        available: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCandidateDiagnostic {
    pub node_id: String,
    pub endpoint: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
    pub rejection_reasons: Vec<RemoteCandidateRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoMatchingRemoteError {
    pub task_label: String,
    pub required: RequiredRemoteDiagnostic,
    pub configured_remote_count: usize,
    pub enabled_remote_count: usize,
    pub enabled_remotes: Vec<RemoteCandidateDiagnostic>,
}

impl NoMatchingRemoteError {
    pub(crate) fn new(
        task_label: String,
        remote: &RemoteSpec,
        configured_remote_count: usize,
        enabled_remote_count: usize,
        enabled_remotes: Vec<RemoteCandidateDiagnostic>,
    ) -> Self {
        Self {
            task_label,
            required: RequiredRemoteDiagnostic::from_spec(remote),
            configured_remote_count,
            enabled_remote_count,
            enabled_remotes,
        }
    }
}

impl std::fmt::Display for NoMatchingRemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "infra error: task {} has no configured remote matching pool={:?} tags={:?} capabilities={:?} transport={}",
            self.task_label,
            self.required.pool,
            self.required.required_tags,
            self.required.required_capabilities,
            self.required.transport_kind.as_result_value()
        )
    }
}

impl std::error::Error for NoMatchingRemoteError {}
