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

        let request: Request = serde_json::from_str(line.trim_end())
            .with_context(|| format!("invalid request line: {}", line.trim_end()))?;
        let response = dispatch_request(request, &manager)?;
        write_protocol_response(&mut writer_half, &response).await?;
    }

    Ok(())
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
