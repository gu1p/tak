use std::path::{Path, PathBuf};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::task::JoinHandle;

mod response;

pub use response::RecordingLeaseConfig;

use super::RecordingEvents;

pub struct RecordingLeaseServer {
    pub socket_path: PathBuf,
    handle: JoinHandle<()>,
}

impl RecordingLeaseServer {
    pub async fn spawn(socket_path: &Path, events: RecordingEvents) -> Self {
        Self::spawn_with_config(socket_path, events, RecordingLeaseConfig::default()).await
    }

    pub async fn spawn_with_config(
        socket_path: &Path,
        events: RecordingEvents,
        config: RecordingLeaseConfig,
    ) -> Self {
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path).expect("bind recording lease socket");
        let socket_path = socket_path.to_path_buf();
        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let events = events.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let response = response::for_request(line.trim_end(), &events, config);
                    let _ = writer.write_all(format!("{response}\n").as_bytes()).await;
                });
            }
        });
        Self {
            socket_path,
            handle,
        }
    }
}

impl Drop for RecordingLeaseServer {
    fn drop(&mut self) {
        self.handle.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
