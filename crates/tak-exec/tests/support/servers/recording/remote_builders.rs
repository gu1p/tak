use tak_proto::NodeStatusResponse;

use super::RecordingEvents;
use super::RecordingRemoteServer;
use super::submit_route::SubmitBehavior;
use super::upload_config::UploadConfig;

impl RecordingRemoteServer {
    pub fn spawn_success(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            None,
        )
    }

    pub fn spawn_success_with_result_delay(
        node_id: &str,
        events: RecordingEvents,
        result_delay: std::time::Duration,
    ) -> Self {
        Self::spawn_with_result_delay(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            None,
            result_delay,
        )
    }

    pub fn spawn_success_with_status(
        node_id: &str,
        events: RecordingEvents,
        status: NodeStatusResponse,
    ) -> Self {
        Self::spawn(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::protocol(),
            Some(status),
        )
    }

    pub fn spawn_submit_failure(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(
            node_id,
            events,
            SubmitBehavior::Failure,
            UploadConfig::protocol(),
            None,
        )
    }

    /// Spawns a node that 404s the workspace-upload routes, forcing the client to inline the
    /// workspace into the submit (the legacy compatibility path).
    pub fn spawn_success_legacy_inline(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::legacy_inline(),
            None,
        )
    }

    /// Spawns a node that reaps each upload right after a submit references it, so a later task
    /// reusing the cached blob gets a 409 and must re-upload — exercises the client fallback.
    pub fn spawn_success_reaping_uploads(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(
            node_id,
            events,
            SubmitBehavior::Success,
            UploadConfig::reaping(),
            None,
        )
    }
}
