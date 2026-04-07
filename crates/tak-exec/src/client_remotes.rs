use std::fs;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use serde::Deserialize;
use tak_core::model::{RemoteSpec, RemoteTransportKind};

use crate::StrictRemoteTarget;

#[derive(Debug, Deserialize)]
struct RemoteInventoryFile {
    remotes: Vec<RemoteRecord>,
}

#[derive(Debug, Deserialize)]
struct RemoteRecord {
    node_id: String,
    base_url: String,
    bearer_token: String,
    #[serde(default)]
    pools: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    transport: String,
    enabled: bool,
}

pub(crate) fn configured_remote_targets(remote: &RemoteSpec) -> Result<Vec<StrictRemoteTarget>> {
    let path = inventory_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    let inventory: RemoteInventoryFile = toml::from_str(&raw)?;
    Ok(inventory
        .remotes
        .into_iter()
        .filter(|candidate| candidate.enabled)
        .filter(|candidate| {
            remote
                .pool
                .as_ref()
                .is_none_or(|pool| candidate.pools.iter().any(|value| value == pool))
        })
        .filter(|candidate| {
            remote
                .required_tags
                .iter()
                .all(|tag| candidate.tags.iter().any(|value| value == tag))
        })
        .filter(|candidate| {
            remote.required_capabilities.iter().all(|capability| {
                candidate
                    .capabilities
                    .iter()
                    .any(|value| value == capability)
            })
        })
        .filter_map(|candidate| {
            let transport_kind = match candidate.transport.as_str() {
                "direct" => RemoteTransportKind::Direct,
                "tor" => RemoteTransportKind::Tor,
                _ => return None,
            };
            (transport_kind == remote.transport_kind).then_some(StrictRemoteTarget {
                node_id: candidate.node_id,
                endpoint: candidate.base_url,
                transport_kind,
                bearer_token: candidate.bearer_token,
                runtime: remote.runtime.clone(),
            })
        })
        .collect::<Vec<_>>())
}

fn inventory_path() -> Result<PathBuf> {
    let root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map_err(|_| anyhow!("failed to resolve config home"))?;
    Ok(root.join("tak").join("remotes.toml"))
}
