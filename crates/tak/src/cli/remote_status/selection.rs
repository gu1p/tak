//! Resolves which configured remotes a status query should target.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use crate::cli::remote_inventory::{RemoteRecord, list_remotes};

pub(super) fn selected_remotes_or_empty_when_daemon_available(
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
