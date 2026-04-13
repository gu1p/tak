use tak_proto::GetTaskResultResponse;

use super::EventPollPlan;

pub(super) struct ScriptedEventsState {
    pub node_id: String,
    pub port: u16,
    pub status_available: bool,
    pub plans: Vec<EventPollPlan>,
    pub fallback_plan: EventPollPlan,
    pub event_calls: usize,
    pub result_ready_after_event_calls: usize,
    pub result: GetTaskResultResponse,
}
