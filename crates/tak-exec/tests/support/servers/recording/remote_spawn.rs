use std::time::Duration;

use tak_proto::NodeStatusResponse;

use super::RecordingEvents;
use super::RecordingRemoteServer;
use super::remote_routes::ResultBehavior;
use super::submit_route::SubmitBehavior;
use super::upload_config::UploadConfig;

impl RecordingRemoteServer {
    pub(super) fn spawn(
        node_id: &str,
        events: RecordingEvents,
        submit: SubmitBehavior,
        upload: UploadConfig,
        status: Option<NodeStatusResponse>,
    ) -> Self {
        Self::spawn_with_result(
            node_id,
            events,
            submit,
            upload,
            status,
            Duration::ZERO,
            ResultBehavior::Success,
        )
    }

    pub(super) fn spawn_with_result_delay(
        node_id: &str,
        events: RecordingEvents,
        submit: SubmitBehavior,
        upload: UploadConfig,
        status: Option<NodeStatusResponse>,
        result_delay: Duration,
    ) -> Self {
        Self::spawn_with_result(
            node_id,
            events,
            submit,
            upload,
            status,
            result_delay,
            ResultBehavior::Success,
        )
    }
}
