use sha2::{Digest, Sha256};
use tak_core::v2::OutputMergeError;
use tak_proto::worker_v2::{
    DispatchDisposition, decode_dispatch_response, encode_dispatch_request,
};

use super::*;

pub(super) async fn send(
    transport: &RemoteAttemptTransport,
    command: &DispatchCommand,
) -> Result<AttemptDispatch> {
    let dispatch = match request::dispatch(&transport.store, command) {
        Ok(dispatch) => dispatch,
        Err(error) => {
            let Some(conflict) = error.downcast_ref::<OutputMergeError>() else {
                return Err(error);
            };
            let message = format!("declared output preparation failed: {conflict}");
            let digest = format!("{:x}", Sha256::digest(message.as_bytes()));
            transport
                .store
                .fail_attempt_permanently(command, &digest, &message)?;
            return Ok(AttemptDispatch::Stale);
        }
    };
    let target = transport.target(command)?;
    let cache = workspace_cache::ensure(transport, &target, &dispatch).await?;
    let cache = match cache {
        tak_proto::worker_v2::WorkspaceCacheDisposition::Hit => "hit",
        tak_proto::worker_v2::WorkspaceCacheDisposition::Miss => "miss",
        tak_proto::worker_v2::WorkspaceCacheDisposition::Stored => {
            bail!("worker cache probe returned stored")
        }
    };
    transport.store.record_worker_cache(command, cache)?;
    let body = encode_dispatch_request(&dispatch.request)?;
    let response = transport
        .broker
        .worker_v2_http_exchange(&target, "POST", "/v2/attempts/dispatch", &body)
        .await?;
    require_status(response.status, &[200, 202], "dispatch")?;
    let response = decode_dispatch_response(&response.body, &command.fencing_token)?;
    match response.disposition {
        DispatchDisposition::Accepted | DispatchDisposition::Duplicate => {
            Ok(AttemptDispatch::Accepted)
        }
        DispatchDisposition::Stale => Ok(AttemptDispatch::Stale),
    }
}
