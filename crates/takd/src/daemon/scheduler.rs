use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerNode {
    pub node_id: String,
    pub cpu_capacity_millis: u64,
    pub cpu_used_millis: u64,
    pub memory_capacity_bytes: u64,
    pub memory_used_bytes: u64,
    pub execution_capacity: u32,
    pub execution_used: u32,
}

impl SchedulerNode {
    #[must_use]
    pub fn with_execution_slots(node_id: impl Into<String>, execution_capacity: u32) -> Self {
        Self {
            node_id: node_id.into(),
            cpu_capacity_millis: u64::MAX,
            cpu_used_millis: 0,
            memory_capacity_bytes: u64::MAX,
            memory_used_bytes: 0,
            execution_capacity,
            execution_used: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchCommand {
    pub run_id: String,
    pub job_id: String,
    pub node_id: String,
    pub authored_attempt: u32,
    pub dispatch_generation: u32,
    pub fencing_token: String,
}
