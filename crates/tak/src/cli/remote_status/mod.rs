use std::io::{IsTerminal, Write, stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use tokio::time::sleep;

use super::remote_inventory::RemoteRecord;

mod fetch;
mod live;
mod render;
mod types;
mod view;

use fetch::fetch_snapshot;
use live::run_remote_status_dashboard;
use render::render_snapshot;

pub(super) use fetch::fetch_snapshot as fetch_remote_status_snapshot;
pub(super) use render::render_snapshot_with_prefix as render_remote_status_snapshot_with_prefix;
pub(super) use types::{DaemonPeerSnapshot, RemoteStatusResult};

pub(super) async fn run_remote_status(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
) -> Result<()> {
    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();
    if stdout().is_terminal() {
        return run_remote_status_dashboard(node_filters, watch, poll_interval, max_polls).await;
    }
    run_remote_status_plain(node_filters, watch, poll_interval, max_polls).await
}

async fn run_remote_status_plain(
    node_filters: &[String],
    watch: bool,
    poll_interval: Duration,
    max_polls: Option<usize>,
) -> Result<()> {
    let mut polls = 0_usize;
    loop {
        let snapshot = fetch_snapshot(node_filters).await?;
        ensure_requested_nodes(node_filters, &snapshot)?;
        print!("{}", render_snapshot(&snapshot));
        stdout().flush().context("flush remote status output")?;

        polls = polls.saturating_add(1);
        if !watch {
            fail_on_remote_errors(&snapshot)?;
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
    }
}

pub(in crate::cli) fn fail_on_remote_errors(snapshot: &[RemoteStatusResult]) -> Result<()> {
    if snapshot.iter().any(|result| result.error.is_some()) {
        bail!("failed to query one or more remote nodes");
    }
    Ok(())
}

fn ensure_requested_nodes(node_filters: &[String], snapshot: &[RemoteStatusResult]) -> Result<()> {
    if !node_filters.is_empty() && snapshot.is_empty() {
        bail!("no enabled remotes matched the requested node filters");
    }
    Ok(())
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
