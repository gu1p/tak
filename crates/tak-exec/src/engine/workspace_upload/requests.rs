use super::*;

pub(super) async fn begin_upload_request(
    target: &StrictRemoteTarget,
    body: &[u8],
) -> Result<(u16, Vec<u8>), RemoteSubmitFailure> {
    for attempt in 0..2 {
        match remote_protocol_http_request(
            target,
            "POST",
            "/v2/workspaces/uploads/begin",
            Some(body),
            "workspace upload begin",
            upload_timeout(),
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(_) if attempt == 0 => continue,
            Err(err) => return Err(submit_transport_error(err)),
        }
    }
    unreachable!("bounded workspace upload begin retry loop returns")
}

pub(super) async fn append_chunk_request(
    target: &StrictRemoteTarget,
    path: &str,
    chunk: &[u8],
) -> Result<(u16, Vec<u8>), RemoteSubmitFailure> {
    for attempt in 0..2 {
        match remote_protocol_http_request(
            target,
            "PATCH",
            path,
            Some(chunk),
            "workspace upload chunk",
            upload_timeout(),
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(_) if attempt == 0 => continue,
            Err(err) => return Err(submit_transport_error(err)),
        }
    }
    unreachable!("bounded workspace upload chunk retry loop returns")
}

pub(super) async fn finish_upload_request(
    target: &StrictRemoteTarget,
    path: &str,
) -> Result<(u16, Vec<u8>), RemoteSubmitFailure> {
    for attempt in 0..2 {
        match remote_protocol_http_request(
            target,
            "POST",
            path,
            None,
            "workspace upload finish",
            upload_timeout(),
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(_) if attempt == 0 => continue,
            Err(err) => return Err(submit_transport_error(err)),
        }
    }
    unreachable!("bounded workspace upload finish retry loop returns")
}
