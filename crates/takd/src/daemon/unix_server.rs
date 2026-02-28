pub async fn run_server(socket_path: &Path, manager: SharedLeaseManager) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create socket directory {}", parent.display()))?;
    }

    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await.with_context(|| {
            format!("failed to remove existing socket {}", socket_path.display())
        })?;
    }

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("failed to bind socket {}", socket_path.display()))?;

    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let manager = Arc::clone(&manager);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, manager).await {
                eprintln!("client handling error: {err}");
            }
        });
    }
}
