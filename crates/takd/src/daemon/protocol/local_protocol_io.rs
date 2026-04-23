use super::*;

pub(super) async fn handle_client(stream: UnixStream, manager: SharedLeaseManager) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let response = match decode_and_dispatch_request(line.trim_end(), &manager) {
            Ok(response) => response,
            Err(err) => protocol_error_response(line.trim_end(), err),
        };
        write_protocol_response(&mut writer_half, &response).await?;
    }

    Ok(())
}

fn decode_and_dispatch_request(
    raw_request: &str,
    manager: &SharedLeaseManager,
) -> Result<Response> {
    let request: Request = serde_json::from_str(raw_request)
        .with_context(|| format!("invalid request line: {raw_request}"))?;
    dispatch_request(request, manager)
}

fn protocol_error_response(raw_request: &str, err: anyhow::Error) -> Response {
    Response::Error {
        request_id: request_id_from_raw_request(raw_request),
        message: format!("{err:#}"),
    }
}

fn request_id_from_raw_request(raw_request: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw_request)
        .ok()
        .and_then(|value| {
            value
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

async fn write_protocol_response(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    response: &Response,
) -> Result<()> {
    let encoded = serde_json::to_string(response)?;
    writer.write_all(encoded.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
