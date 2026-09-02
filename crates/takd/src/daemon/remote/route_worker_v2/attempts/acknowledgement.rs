use tak_proto::worker_v2::{AckAttemptResponse, decode_ack_request, encode_ack_response};

use super::super::super::*;

pub(super) fn acknowledge(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    let request = match decode_ack_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_worker_acknowledgement")),
    };
    if context.node_info()?.node_id != request.identity.node_id {
        return Ok(text_response(409, "node_identity_mismatch"));
    }
    store.acknowledge_worker_v2_terminal_for_run(
        &request.identity,
        &request.terminal_digest,
        request.run_terminal,
    )?;
    let response = AckAttemptResponse {
        protocol_version: 2,
        fencing_token: request.identity.fencing_token,
        terminal_digest: request.terminal_digest,
        acknowledged: true,
    };
    Ok(binary_response(
        200,
        "application/json",
        encode_ack_response(&response)?,
    ))
}
