use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;

use super::{DaemonPeerSnapshot, RemoteRecord, RemoteStatusResult};

const DAEMON_PEERS_TIMEOUT: Duration = Duration::from_millis(500);

/// Outcome of querying the local `takd serve` peer list for Tor node status.
///
/// The distinction matters: `Snapshot` means the local daemon answered — even an
/// empty vector is the definitive "takd is up but is not currently reporting any
/// matching Tor peer", which is a completely different situation from
/// `Unavailable` ("the takd peer socket did not answer at all"). Collapsing the
/// two into a single `None` is what made a running `takd` look like it was down
/// and produced the misleading "requires local takd serve" message.
pub(in crate::cli) enum DaemonPeerOutcome {
    /// The local takd peer socket did not answer: takd is not running, was too
    /// busy to reply within [`DAEMON_PEERS_TIMEOUT`], or `TAKD_SOCKET` points
    /// somewhere else. Tor node status cannot be read.
    Unavailable,
    /// The local takd answered; these are the matching Tor peer results, which
    /// may be empty when no configured Tor peer is currently connected.
    Snapshot(Vec<RemoteStatusResult>),
}

impl DaemonPeerOutcome {
    /// Whether the local takd answered the peer query at all.
    ///
    /// ```no_run
    /// # // Reason: This private CLI outcome is exercised through remote-status tests.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(in crate::cli) fn daemon_reachable(&self) -> bool {
        matches!(self, Self::Snapshot(_))
    }
}

pub(in crate::cli) async fn fetch_daemon_peer_snapshot(
    node_filters: &[String],
) -> Result<DaemonPeerOutcome> {
    let socket_path = daemon_socket_path();
    let peers = match fetch_daemon_peers(&socket_path).await {
        Ok(peers) => peers,
        Err(DaemonPeerFetchError::Unavailable) => return Ok(DaemonPeerOutcome::Unavailable),
        Err(DaemonPeerFetchError::Failed(err)) => return Err(err),
    };
    Ok(DaemonPeerOutcome::Snapshot(results_from_peers(
        peers,
        node_filters,
    )))
}

pub(super) fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}

/// Honest replacement for the old "Tor remote status requires local takd serve"
/// bail. Tor `.onion` node status is owned by `takd serve`; this is only ever
/// shown when the local takd peer socket is genuinely unreachable (so it no
/// longer fires while takd is running and answering).
///
/// ```no_run
/// # // Reason: This private CLI message depends on runtime socket paths.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn tor_status_daemon_unreachable_message() -> String {
    format!(
        "Tor node status comes from local takd serve; local takd peer socket at {} is unreachable (start takd serve or set TAKD_SOCKET)",
        daemon_socket_path().display()
    )
}

async fn fetch_daemon_peers(
    socket_path: &Path,
) -> std::result::Result<Vec<DaemonPeerSnapshot>, DaemonPeerFetchError> {
    match timeout(DAEMON_PEERS_TIMEOUT, fetch_daemon_peers_inner(socket_path)).await {
        Ok(result) => result,
        Err(_) => Err(DaemonPeerFetchError::Unavailable),
    }
}

async fn fetch_daemon_peers_inner(
    socket_path: &Path,
) -> std::result::Result<Vec<DaemonPeerSnapshot>, DaemonPeerFetchError> {
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|_| DaemonPeerFetchError::Unavailable)?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half
        .write_all(br#"{"type":"PeersList","request_id":"peers"}"#)
        .await
        .map_err(DaemonPeerFetchError::from)?;
    writer_half
        .write_all(b"\n")
        .await
        .map_err(DaemonPeerFetchError::from)?;
    writer_half
        .flush()
        .await
        .map_err(DaemonPeerFetchError::from)?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    if reader
        .read_line(&mut line)
        .await
        .map_err(DaemonPeerFetchError::from)?
        == 0
    {
        return Err(DaemonPeerFetchError::Failed(anyhow::anyhow!(
            "daemon closed connection before peers response"
        )));
    }
    match serde_json::from_str::<DaemonResponse>(line.trim_end())
        .map_err(DaemonPeerFetchError::from)?
    {
        DaemonResponse::PeersSnapshot { peers } => Ok(peers),
        DaemonResponse::Error { message } => {
            Err(DaemonPeerFetchError::Failed(anyhow::anyhow!(message)))
        }
        DaemonResponse::Other => Err(DaemonPeerFetchError::Failed(anyhow::anyhow!(
            "unexpected daemon peers response"
        ))),
    }
}

fn results_from_peers(
    peers: Vec<DaemonPeerSnapshot>,
    node_filters: &[String],
) -> Vec<RemoteStatusResult> {
    let wanted = node_filters
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    let mut results = peers
        .into_iter()
        .filter(|peer| peer.transport == "tor")
        .filter(|peer| wanted.is_empty() || wanted.contains(peer.node_id.as_str()))
        .map(result_from_peer)
        .collect::<Vec<_>>();
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

fn result_from_peer(peer: DaemonPeerSnapshot) -> RemoteStatusResult {
    let error = peer_failure_status(&peer).map(str::to_string);
    RemoteStatusResult {
        remote: RemoteRecord {
            node_id: peer.node_id.clone(),
            display_name: peer.display_name.clone(),
            base_url: peer.endpoint.clone(),
            bearer_token: String::new(),
            pools: peer.pools.clone(),
            tags: peer.tags.clone(),
            capabilities: peer.capabilities.clone(),
            transport: peer.transport.clone(),
            enabled: true,
        },
        status: None,
        error,
        peer: Some(peer),
    }
}

fn peer_failure_status(peer: &DaemonPeerSnapshot) -> Option<&'static str> {
    match peer.state.as_str() {
        "auth_failed" => Some("auth_failed"),
        "unreachable" => Some("unreachable"),
        "protocol_mismatch" => Some("protocol_mismatch"),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DaemonResponse {
    PeersSnapshot {
        peers: Vec<DaemonPeerSnapshot>,
    },
    Error {
        message: String,
    },
    #[serde(other)]
    Other,
}

enum DaemonPeerFetchError {
    Unavailable,
    Failed(anyhow::Error),
}

impl From<anyhow::Error> for DaemonPeerFetchError {
    fn from(value: anyhow::Error) -> Self {
        Self::Failed(value)
    }
}

impl From<std::io::Error> for DaemonPeerFetchError {
    fn from(value: std::io::Error) -> Self {
        Self::Failed(anyhow::Error::new(value).context("daemon peer request failed"))
    }
}

impl From<serde_json::Error> for DaemonPeerFetchError {
    fn from(value: serde_json::Error) -> Self {
        Self::Failed(anyhow::Error::new(value).context("decode daemon peer response"))
    }
}
