use tak_core::model::TaskLabel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TaskActivity {
    Waiting,
    Placing,
    Staging,
    Uploading,
    Queued,
    Running,
    Retrying,
    Syncing,
    Passed,
    Failed,
    Cancelled,
}

impl TaskActivity {
    pub(super) fn is_terminal(self) -> bool {
        matches!(self, Self::Passed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug)]
pub(super) struct TaskRow {
    pub(super) label: TaskLabel,
    pub(super) member_count: usize,
    pub(super) activity: TaskActivity,
    pub(super) node: Option<String>,
    pub(super) transport: Option<String>,
    pub(super) queue_id: Option<String>,
    pub(super) queue_position: Option<usize>,
    pub(super) attempt: u32,
    pub(super) task_run_id: Option<String>,
    pub(super) exit_code: Option<i32>,
    pub(super) started_at: std::time::Instant,
    pub(super) finished_elapsed: Option<std::time::Duration>,
}
