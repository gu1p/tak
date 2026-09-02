use super::super::*;

pub(super) fn handle(context: &RemoteNodeContext, method: &str) -> WorkerHttpResponse {
    if method != "GET" {
        return text_response(405, "method_not_allowed");
    }
    match context.node_info().and_then(|node| {
        tak_proto::worker_v2::encode_identity(&tak_proto::worker_v2::WorkerIdentity {
            protocol_version: tak_proto::worker_v2::PROTOCOL_VERSION,
            node_id: node.node_id,
            display_name: node.display_name,
            base_url: node.base_url,
            pools: node.pools,
            tags: node.tags,
            capabilities: node.capabilities,
            transport: node.transport,
        })
    }) {
        Ok(body) => binary_response(200, "application/json", body),
        Err(error) => {
            tracing::error!(error = %error, "failed to encode worker v2 identity");
            text_response(500, "identity_unavailable")
        }
    }
}
