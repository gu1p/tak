use super::RecordingEvents;
use super::RecordingRemoteServer;
use super::remote_routes::ResultBehavior;
use super::submit_route::SubmitBehavior;
use super::upload_config::UploadConfig;

impl RecordingRemoteServer {
    pub fn spawn_confirmed_container_oom_137(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn_with_result(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            None,
            std::time::Duration::ZERO,
            ResultBehavior::ContainerOom137,
        )
    }

    pub fn spawn_legacy_infrastructure_137(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn_with_result(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            None,
            std::time::Duration::ZERO,
            ResultBehavior::LegacyInfrastructure137,
        )
    }

    pub fn spawn_task_exit_1(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn_with_result(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            None,
            std::time::Duration::ZERO,
            ResultBehavior::TaskExit1,
        )
    }
}
