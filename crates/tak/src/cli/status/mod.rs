use std::collections::BTreeSet;
use std::io::{Write, stdout};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::time::sleep;

use super::remote_inventory::{RemoteRecord, list_remotes};
use super::remote_status::{
    DaemonPeerStatusOutcome, RemoteStatusResult, fetch_daemon_peer_status_snapshot,
    fetch_mixed_remote_status_snapshot, fetch_remote_status_snapshot,
};

mod daemon;
mod local;
mod render;

use local::local_status_snapshot;
use render::{render_local_snapshot, render_status_snapshot};

pub(super) async fn run_local_status(watch: bool, interval_ms: u64) -> Result<()> {
    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();
    let mut polls = 0_usize;

    loop {
        let snapshot = local_status_snapshot().await?;
        print!("{}", render_local_snapshot(&snapshot));
        stdout().flush().context("flush local status output")?;

        polls = polls.saturating_add(1);
        if !watch || max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
    }
}

pub(super) async fn run_status(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
) -> Result<()> {
    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();
    let mut polls = 0_usize;

    loop {
        let local = local_status_snapshot().await?;
        let daemon_outcome = fetch_daemon_peer_status_snapshot(node_filters).await?;
        let remotes = selected_status_remotes_or_empty_when_daemon_available(
            node_filters,
            daemon_outcome.daemon_reachable(),
        )?;
        let remote = match daemon_outcome {
            DaemonPeerStatusOutcome::Snapshot(snapshot) => {
                fetch_mixed_remote_status_snapshot(&remotes, snapshot).await
            }
            DaemonPeerStatusOutcome::Unavailable => fetch_remote_status_snapshot(&remotes).await,
        };
        print!("{}", render_status_snapshot(&local, &remote));
        stdout().flush().context("flush status output")?;

        polls = polls.saturating_add(1);
        if !watch {
            fail_on_remote_errors(&remote)?;
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
    }
}

fn fail_on_remote_errors(remote: &[RemoteStatusResult]) -> Result<()> {
    if remote.iter().any(|result| result.error.is_some()) {
        bail!("failed to query one or more remote nodes");
    }
    Ok(())
}

fn selected_status_remotes_or_empty_when_daemon_available(
    node_filters: &[String],
    daemon_available: bool,
) -> Result<Vec<RemoteRecord>> {
    match selected_status_remotes(node_filters) {
        Ok(remotes) => Ok(remotes),
        Err(_) if daemon_available => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn selected_status_remotes(node_filters: &[String]) -> Result<Vec<RemoteRecord>> {
    let enabled = list_remotes()?
        .into_iter()
        .filter(|remote| remote.enabled)
        .collect::<Vec<_>>();
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
    std::env::var("TAK_TEST_STATUS_MAX_POLLS")
        .or_else(|_| std::env::var("TAK_TEST_REMOTE_STATUS_MAX_POLLS"))
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}
