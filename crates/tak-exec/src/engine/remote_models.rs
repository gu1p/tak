#[derive(Debug, Clone)]
struct StrictRemoteTarget {
    node_id: String,
    endpoint: String,
    transport_kind: RemoteTransportKind,
    bearer_token: String,
    runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone)]
struct TaskPlacement {
    placement_mode: PlacementMode,
    remote_node_id: Option<String>,
    strict_remote_target: Option<StrictRemoteTarget>,
    ordered_remote_targets: Vec<StrictRemoteTarget>,
    decision_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteProtocolResult {
    success: bool,
    exit_code: Option<i32>,
    synced_outputs: Vec<SyncedOutput>,
    runtime_kind: Option<String>,
    runtime_engine: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedRemoteEvents {
    next_seq: u64,
    done: bool,
    remote_logs: Vec<RemoteLogChunk>,
}

#[derive(Debug)]
struct RemoteWorkspaceStage {
    temp_dir: tempfile::TempDir,
    manifest_hash: String,
    archive_zip_base64: String,
}

#[derive(Debug, Clone)]
struct RuntimeExecutionMetadata {
    kind: String,
    engine: Option<String>,
    env_overrides: BTreeMap<String, String>,
    container_plan: Option<ContainerExecutionPlan>,
}

#[derive(Debug, Clone)]
struct ContainerExecutionPlan {
    engine: ContainerEngine,
    image: String,
}

#[derive(Debug, Clone, Copy)]
struct RemoteSubmitContext<'a> {
    task_run_id: &'a str,
    attempt: u32,
    task_label: &'a str,
    remote_workspace: &'a RemoteWorkspaceStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerLifecycleStage {
    Pull,
    Start,
    Runtime,
}

impl ContainerLifecycleStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Start => "start",
            Self::Runtime => "runtime",
        }
    }
}
