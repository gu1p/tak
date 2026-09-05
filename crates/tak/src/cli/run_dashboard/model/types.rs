use std::collections::BTreeMap;

pub(in crate::cli) struct DashboardSeed {
    pub(in crate::cli::run_dashboard) run_id: String,
    pub(in crate::cli::run_dashboard) lifecycle: String,
    pub(in crate::cli::run_dashboard) max_parallel_jobs: u32,
    pub(in crate::cli::run_dashboard) jobs: Vec<DashboardJobSeed>,
}

pub(in crate::cli::run_dashboard) struct DashboardJobSeed {
    pub(in crate::cli::run_dashboard) job_id: String,
    pub(in crate::cli::run_dashboard) task_ids: Vec<String>,
    pub(in crate::cli::run_dashboard) state: String,
    pub(in crate::cli::run_dashboard) node_id: Option<String>,
    pub(in crate::cli::run_dashboard) candidate_node_ids: Vec<String>,
    pub(in crate::cli::run_dashboard) queue: Option<String>,
    pub(in crate::cli::run_dashboard) attempt: u32,
    pub(in crate::cli::run_dashboard) cache: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cli::run_dashboard) enum JobActivity {
    Unknown,
    Staging,
    Blocked,
    Ready,
    Transferring,
    Running,
    Retrying,
    OutputCommitting,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl JobActivity {
    pub(in crate::cli::run_dashboard) fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Staging => "staging",
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::Transferring => "transferring",
            Self::Running => "running",
            Self::Retrying => "retrying",
            Self::OutputCommitting => "output committing",
            Self::Cancelling => "cancelling",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }

    pub(in crate::cli::run_dashboard) fn is_active(self) -> bool {
        matches!(
            self,
            Self::Transferring | Self::Running | Self::OutputCommitting | Self::Cancelling
        )
    }

    pub(in crate::cli::run_dashboard) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Skipped
        )
    }

    pub(super) fn from_state(state: &str) -> Self {
        match state {
            "staged" | "staging" => Self::Staging,
            "blocked" => Self::Blocked,
            "ready" | "queued" => Self::Ready,
            "transferring" => Self::Transferring,
            "running" => Self::Running,
            "retrying" => Self::Retrying,
            "output_committing" => Self::OutputCommitting,
            "cancelling" => Self::Cancelling,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "skipped" => Self::Skipped,
            _ => Self::Unknown,
        }
    }
}

pub(in crate::cli::run_dashboard) struct DashboardJob {
    pub(in crate::cli::run_dashboard) task_ids: Vec<String>,
    pub(in crate::cli::run_dashboard) activity: JobActivity,
    pub(in crate::cli::run_dashboard) node_id: Option<String>,
    pub(in crate::cli::run_dashboard) attempt: u32,
    pub(in crate::cli::run_dashboard) cache: Option<String>,
    pub(in crate::cli::run_dashboard) candidate_node_ids: Vec<String>,
    pub(in crate::cli::run_dashboard) queue: Option<String>,
}

#[derive(Default)]
pub(in crate::cli::run_dashboard) struct NodeLane {
    pub(in crate::cli::run_dashboard) active_jobs: Vec<String>,
    pub(in crate::cli::run_dashboard) candidate_queue: Vec<NodeQueueEntry>,
}

pub(in crate::cli::run_dashboard) struct NodeQueueEntry {
    pub(in crate::cli::run_dashboard) task: String,
    pub(in crate::cli::run_dashboard) queue: Option<String>,
}

pub(in crate::cli::run_dashboard) struct LogLine {
    pub(in crate::cli::run_dashboard) job: String,
    pub(in crate::cli::run_dashboard) node: String,
    pub(in crate::cli::run_dashboard) text: String,
}

pub(in crate::cli::run_dashboard) struct DashboardState {
    pub(in crate::cli::run_dashboard) run_id: String,
    pub(in crate::cli::run_dashboard) lifecycle: String,
    pub(in crate::cli::run_dashboard) max_parallel_jobs: u32,
    pub(in crate::cli::run_dashboard) jobs: BTreeMap<String, DashboardJob>,
    pub(in crate::cli::run_dashboard) nodes: BTreeMap<String, NodeLane>,
    pub(in crate::cli::run_dashboard) logs: Vec<LogLine>,
    pub(in crate::cli::run_dashboard) diagnostics: Vec<String>,
    pub(in crate::cli::run_dashboard) error: Option<String>,
    pub(in crate::cli::run_dashboard) notice: Option<String>,
    pub(super) last_applied_event: Option<u64>,
}
