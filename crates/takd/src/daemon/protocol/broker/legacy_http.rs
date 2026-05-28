use super::*;

pub(super) async fn read_remote_http_response<R>(
    remote: &mut R,
) -> std::result::Result<Vec<u8>, BrokerHttpError>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let mut response = Vec::new();
    let Some(header_end) = read_response_headers(remote, &mut response).await? else {
        return Ok(response);
    };
    let Some(content_length) = response_content_length(&response[..header_end])? else {
        read_response_body_until_eof(remote, &mut response, header_end).await?;
        return Ok(response);
    };
    read_response_body(remote, &mut response, header_end, content_length).await?;
    Ok(response)
}

async fn read_response_headers<R>(
    remote: &mut R,
    response: &mut Vec<u8>,
) -> std::result::Result<Option<usize>, BrokerHttpError>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let mut chunk = [0_u8; 1024];
    loop {
        let read = remote
            .read(&mut chunk)
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("read_failed", err))?;
        if read == 0 {
            return Ok(None);
        }
        let previous_len = response.len();
        response.extend_from_slice(&chunk[..read]);
        let search_start = previous_len.saturating_sub(3);
        if let Some(index) = response[search_start..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            return Ok(Some(search_start + index + 4));
        }
        if response.len() > MAX_RESPONSE_HEADER_BYTES {
            return Err(BrokerHttpError::bad_gateway(
                "read_failed",
                "response headers too large",
            ));
        }
    }
}

async fn read_response_body<R>(
    remote: &mut R,
    response: &mut Vec<u8>,
    header_end: usize,
    content_length: usize,
) -> std::result::Result<(), BrokerHttpError>
where
    R: AsyncRead + Unpin + ?Sized,
{
    ensure_response_body_size(content_length)?;
    let response_len = header_end.saturating_add(content_length);
    if response.len() < response_len {
        let current_len = response.len();
        response.resize(response_len, 0);
        remote
            .read_exact(&mut response[current_len..response_len])
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("read_failed", err))?;
    }
    response.truncate(response_len);
    Ok(())
}

async fn read_response_body_until_eof<R>(
    remote: &mut R,
    response: &mut Vec<u8>,
    header_end: usize,
) -> std::result::Result<(), BrokerHttpError>
where
    R: AsyncRead + Unpin + ?Sized,
{
    ensure_response_body_size(response.len().saturating_sub(header_end))?;
    let mut chunk = [0_u8; 8192];
    loop {
        let read = remote
            .read(&mut chunk)
            .await
            .map_err(|err| BrokerHttpError::bad_gateway("read_failed", err))?;
        if read == 0 {
            return Ok(());
        }
        ensure_response_body_size(response.len().saturating_sub(header_end) + read)?;
        response.extend_from_slice(&chunk[..read]);
    }
}

fn response_content_length(headers: &[u8]) -> std::result::Result<Option<usize>, BrokerHttpError> {
    for line in String::from_utf8_lossy(headers).lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("content-length") {
            continue;
        }
        return value
            .trim()
            .parse::<usize>()
            .map(Some)
            .map_err(|err| BrokerHttpError::bad_gateway("invalid_content_length", err));
    }
    Ok(None)
}

fn ensure_response_body_size(size: usize) -> std::result::Result<(), BrokerHttpError> {
    if size > MAX_RESPONSE_BODY_BYTES {
        return Err(BrokerHttpError::bad_gateway(
            "response_body_too_large",
            format!("response body exceeds {MAX_RESPONSE_BODY_BYTES} bytes"),
        ));
    }
    Ok(())
}
