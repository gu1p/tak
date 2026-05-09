use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Clone, Debug)]
pub(super) enum LocalDaemonStatus {
    Available(LocalDaemonSnapshot),
    Unavailable { detail: String },
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct LocalDaemonSnapshot {
    pub(super) active_leases: usize,
    pub(super) pending_requests: usize,
    #[serde(default)]
    pub(super) usage: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DaemonResponse {
    StatusSnapshot {
        status: LocalDaemonSnapshot,
    },
    Error {
        message: String,
    },
    #[serde(other)]
    Other,
}

pub(super) async fn local_daemon_status() -> LocalDaemonStatus {
    let Some(socket_path) = std::env::var_os("TAKD_SOCKET").map(PathBuf::from) else {
        return LocalDaemonStatus::Unavailable {
            detail: "TAKD_SOCKET not set".to_string(),
        };
    };

    match fetch_daemon_status(&socket_path).await {
        Ok(status) => LocalDaemonStatus::Available(status),
        Err(err) => LocalDaemonStatus::Unavailable {
            detail: err.to_string(),
        },
    }
}

async fn fetch_daemon_status(socket_path: &Path) -> anyhow::Result<LocalDaemonSnapshot> {
    let stream = UnixStream::connect(socket_path).await?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half
        .write_all(br#"{"type":"Status","request_id":"tak-status"}"#)
        .await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        anyhow::bail!("daemon closed connection before response");
    }
    match serde_json::from_str::<DaemonResponse>(line.trim_end())? {
        DaemonResponse::StatusSnapshot { status } => Ok(status),
        DaemonResponse::Error { message } => anyhow::bail!("{message}"),
        DaemonResponse::Other => anyhow::bail!("unexpected daemon status response"),
    }
}
