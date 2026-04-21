use super::*;

/// Converts core model need definitions into daemon wire-format needs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn convert_needs(needs: &[NeedDef]) -> Vec<NeedRequest> {
    needs
        .iter()
        .map(|need| NeedRequest {
            name: need.limiter.name.clone(),
            scope: need.limiter.scope.clone(),
            scope_key: need.limiter.scope_key.clone(),
            slots: need.slots,
        })
        .collect()
}

/// Sends one NDJSON request to the daemon and returns the decoded response frame.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) async fn send_daemon_request(socket_path: &Path, request: Request) -> Result<Response> {
    let stream = UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();

    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before response");
    }

    serde_json::from_str(line.trim_end()).context("failed to decode daemon response")
}
