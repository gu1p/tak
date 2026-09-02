use std::io::{Write, stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::sleep;

use super::remote_status::{fail_on_remote_errors, fetch_remote_status_snapshot};

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
        let remote = fetch_remote_status_snapshot(node_filters).await?;
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

fn test_max_polls() -> Option<usize> {
    std::env::var("TAK_TEST_STATUS_MAX_POLLS")
        .or_else(|_| std::env::var("TAK_TEST_REMOTE_STATUS_MAX_POLLS"))
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}
