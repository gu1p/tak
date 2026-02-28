/// Requests daemon status over the Unix socket protocol.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn query_daemon_status(socket_path: PathBuf) -> Result<takd::StatusSnapshot> {
    let stream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let request = Request::Status(StatusRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before responding");
    }

    match serde_json::from_str::<Response>(line.trim_end())? {
        Response::StatusSnapshot { status, .. } => Ok(status),
        Response::Error { message, .. } => bail!("daemon error: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}
