use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::io::{Write, stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

use super::remote_inventory::{RemoteRecord, list_remotes};

mod fetch;
mod http;
mod render;

use fetch::fetch_snapshot;
use render::render_snapshot;

pub(super) async fn run_remote_status(
    node_filters: &[String],
    watch: bool,
    interval_ms: u64,
) -> Result<()> {
    let remotes = selected_remotes(node_filters)?;
    let poll_interval = Duration::from_millis(interval_ms.max(1));
    let max_polls = test_max_polls();
    let mut polls = 0_usize;

    loop {
        let snapshot = fetch_snapshot(&remotes).await;
        if watch {
            print!("\x1b[2J\x1b[H");
        }
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

#[derive(Clone)]
pub(super) struct RemoteStatusResult {
    pub(super) remote: RemoteRecord,
    pub(super) status: Option<tak_proto::NodeStatusResponse>,
    pub(super) error: Option<String>,
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
