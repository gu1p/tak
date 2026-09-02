use super::super::*;

pub(super) fn handle(context: &RemoteNodeContext, method: &str) -> WorkerHttpResponse {
    if method != "GET" {
        return text_response(405, "method_not_allowed");
    }
    match context.worker_v2_snapshot() {
        Ok(snapshot) => match tak_proto::worker_v2::encode_snapshot(&snapshot) {
            Ok(body) => binary_response(200, "application/json", body),
            Err(error) => {
                tracing::error!(error = %error, "failed to encode worker v2 snapshot");
                text_response(500, "snapshot_unavailable")
            }
        },
        Err(error) => {
            tracing::error!(error = %error, "failed to build worker v2 snapshot");
            text_response(500, "snapshot_unavailable")
        }
    }
}
