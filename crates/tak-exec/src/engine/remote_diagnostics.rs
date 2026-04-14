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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemotePreflightFailureKind {
    Timeout,
    Auth,
    HttpStatus,
    InvalidMetadata,
    Unhealthy,
    Connect,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePreflightFailure {
    pub node_id: String,
    pub endpoint: String,
    pub transport: String,
    pub kind: RemotePreflightFailureKind,
    pub message: String,
    pub live_transport_state: Option<String>,
    pub live_transport_detail: Option<String>,
    pub last_observation: Option<RemoteObservation>,
}

impl std::fmt::Display for RemotePreflightFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePreflightExhaustedError {
    pub task_label: String,
    pub failures: Vec<RemotePreflightFailure>,
}

impl std::fmt::Display for RemotePreflightExhaustedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.failures.is_empty() {
            write!(
                f,
                "infra error: no reachable remote fallback candidates for task {}",
                self.task_label
            )
        } else {
            let failures = self
                .failures
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            write!(
                f,
                "infra error: no reachable remote fallback candidates for task {}: {}",
                self.task_label, failures
            )
        }
    }
}

impl std::error::Error for RemotePreflightExhaustedError {}
