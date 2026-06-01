use crate::engine::{RemoteLogChunk, TaskStatusEventKind};

#[derive(Debug, Clone)]
pub(crate) struct ParsedRemoteEvents {
    pub(crate) next_seq: u64,
    pub(crate) done: bool,
    pub(crate) remote_logs: Vec<RemoteLogChunk>,
    pub(crate) status_messages: Vec<String>,
    pub(crate) status_updates: Vec<RemoteStatusUpdate>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteStatusUpdate {
    pub(crate) message: String,
    pub(crate) kind: TaskStatusEventKind,
    pub(crate) queue_position: Option<usize>,
}
