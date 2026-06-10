use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::io::{IsTerminal, Write, stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

use super::remote_inventory::{RemoteRecord, list_remotes};

mod daemon;
mod fetch;
mod http;
mod live;
mod render;
mod types;
mod view;

use daemon::{DaemonPeerOutcome, fetch_daemon_peer_snapshot};
use fetch::fetch_snapshot;
use live::run_remote_status_dashboard;
use render::render_snapshot;

pub(in crate::cli) use daemon::DaemonPeerOutcome as DaemonPeerStatusOutcome;
pub(super) use daemon::fetch_daemon_peer_snapshot as fetch_daemon_peer_status_snapshot;
pub(super) use fetch::fetch_snapshot as fetch_remote_status_snapshot;
pub(super) use render::render_snapshot_with_prefix as render_remote_status_snapshot_with_prefix;
pub(super) use types::{DaemonPeerSnapshot, RemoteStatusResult};

pub(super) async fn run_remote_status(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
) -> Result<()> {
    let daemon_outcome = fetch_daemon_peer_snapshot(node_filters).await?;
    let remotes = selected_remotes_or_empty_when_daemon_available(
        node_filters,
        daemon_outcome.daemon_reachable(),
    )?;
    if let DaemonPeerOutcome::Snapshot(snapshot) = daemon_outcome {
        return run_remote_status_daemon_plain(
            node_filters,
            watch,
            interval_ms,
            &remotes,
            snapshot,
        )
        .await;
    }

    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();

    if stdout().is_terminal() {
        return run_remote_status_dashboard(&remotes, watch, poll_interval, max_polls).await;
    }

    run_remote_status_plain(&remotes, watch, poll_interval, max_polls).await
}

async fn run_remote_status_daemon_plain(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
    remotes: &[RemoteRecord],
    initial_snapshot: Vec<RemoteStatusResult>,
) -> Result<()> {
    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();
    let mut polls = 0_usize;

    loop {
        let snapshot = if polls == 0 {
            fetch_mixed_remote_status_snapshot(remotes, initial_snapshot.clone()).await
        } else {
            let daemon_snapshot = match fetch_daemon_peer_snapshot(node_filters).await? {
                DaemonPeerOutcome::Snapshot(snapshot) => snapshot,
                DaemonPeerOutcome::Unavailable => Vec::new(),
            };
            fetch_mixed_remote_status_snapshot(remotes, daemon_snapshot).await
        };
        print!("{}", render_snapshot(&snapshot));
        stdout().flush().context("flush remote status output")?;

        polls = polls.saturating_add(1);
        if !watch {
            if snapshot.iter().any(|result| result.error.is_some()) {
                bail!("failed to query one or more remote nodes");
            }
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
    }
}

pub(in crate::cli) async fn fetch_mixed_remote_status_snapshot(
    remotes: &[RemoteRecord],
    daemon_snapshot: Vec<RemoteStatusResult>,
) -> Vec<RemoteStatusResult> {
    let direct_remotes = remotes
        .iter()
        .filter(|remote| remote.transport != "tor")
        .cloned()
        .collect::<Vec<_>>();
    let mut results = fetch_snapshot(&direct_remotes).await;
    // The local takd answered. Any configured Tor remote it is *not* reporting
    // is genuinely not connected — surface it with an honest status instead of
    // silently dropping it (or falling back to the misleading direct-probe stub).
    let reported = daemon_snapshot
        .iter()
        .map(|result| result.remote.node_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut missing_tor = remotes
        .iter()
        .filter(|remote| remote.transport == "tor")
        .filter(|remote| !reported.contains(remote.node_id.as_str()))
        .map(|remote| tor_peer_not_reported_result(remote.clone()))
        .collect::<Vec<_>>();
    results.extend(daemon_snapshot);
    results.append(&mut missing_tor);
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

/// A Tor remote that is configured but absent from the local takd peer snapshot:
/// takd is up, it just has no live session for this node yet.
fn tor_peer_not_reported_result(remote: RemoteRecord) -> RemoteStatusResult {
    RemoteStatusResult {
        remote,
        status: None,
        error: Some(
            "not reported by local takd peer manager (Tor peer not connected; takd serve may still be establishing the session)"
                .to_string(),
        ),
        peer: None,
    }
}

async fn run_remote_status_plain(
    remotes: &[RemoteRecord],
    watch: bool,
    poll_interval: Duration,
    max_polls: Option<usize>,
) -> Result<()> {
    let mut polls = 0_usize;

    loop {
        let snapshot = fetch_snapshot(remotes).await;
        print!("{}", render_snapshot(&snapshot));
        stdout().flush().context("flush remote status output")?;

        polls = polls.saturating_add(1);
        if !watch {
            if snapshot.iter().any(|result| result.error.is_some()) {
                bail!("failed to query one or more remote nodes");
            }
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
    }
}

fn selected_remotes_or_empty_when_daemon_available(
    node_filters: &[String],
    daemon_available: bool,
) -> Result<Vec<RemoteRecord>> {
    match selected_remotes(node_filters) {
        Ok(remotes) => Ok(remotes),
        Err(_) if daemon_available => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn selected_remotes(node_filters: &[String]) -> Result<Vec<RemoteRecord>> {
    let enabled = list_remotes()?
        .into_iter()
        .filter(|remote| remote.enabled)
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        bail!("no enabled remotes configured");
    }
    if node_filters.is_empty() {
        return Ok(enabled);
    }

    let wanted = node_filters
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    let selected = enabled
        .into_iter()
        .filter(|remote| wanted.contains(remote.node_id.as_str()))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        bail!("no enabled remotes matched the requested node filters");
    }
    Ok(selected)
}

fn test_max_polls() -> Option<usize> {
    std::env::var("TAK_TEST_REMOTE_STATUS_MAX_POLLS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

fn unix_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod view_tests;
