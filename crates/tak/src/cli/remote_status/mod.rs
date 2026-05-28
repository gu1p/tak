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

use daemon::fetch_daemon_peer_snapshot;
use fetch::fetch_snapshot;
use live::run_remote_status_dashboard;
use render::render_snapshot;

pub(super) use daemon::fetch_daemon_peer_snapshot as fetch_daemon_peer_status_snapshot;
pub(super) use fetch::fetch_snapshot as fetch_remote_status_snapshot;
pub(super) use render::render_snapshot_with_prefix as render_remote_status_snapshot_with_prefix;
pub(super) use types::{DaemonPeerSnapshot, RemoteStatusResult};

pub(super) async fn run_remote_status(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
) -> Result<()> {
    let daemon_snapshot = fetch_daemon_peer_snapshot(node_filters).await?;
    let remotes =
        selected_remotes_or_empty_when_daemon_available(node_filters, daemon_snapshot.is_some())?;
    if let Some(snapshot) = daemon_snapshot {
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
            let daemon_snapshot = fetch_daemon_peer_snapshot(node_filters)
                .await?
                .unwrap_or_default();
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
    results.extend(daemon_snapshot);
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
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
