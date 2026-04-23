use std::fs;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use serde::Deserialize;
use tak_core::model::RemoteSpec;

use crate::{
    RemoteCandidateDiagnostic, RemoteCandidateRejection, RemoteTargetSelection, StrictRemoteTarget,
    engine::remote_models::StrictRemoteTransportKind,
};

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

pub(crate) fn configured_remote_targets(remote: &RemoteSpec) -> Result<RemoteTargetSelection> {
    let path = inventory_path()?;
    if !path.exists() {
        return Ok(RemoteTargetSelection {
            configured_remote_count: 0,
            enabled_remote_count: 0,
            enabled_remotes: Vec::new(),
            matched_targets: Vec::new(),
        });
    }
    let raw = fs::read_to_string(path)?;
    let inventory: RemoteInventoryFile = toml::from_str(&raw)?;
    let configured_remote_count = inventory.remotes.len();
    let mut enabled_remote_count = 0;
    let mut enabled_remotes = Vec::new();
    let mut matched_targets = Vec::new();

    for candidate in inventory
        .remotes
        .into_iter()
        .filter(|candidate| candidate.enabled)
    {
        enabled_remote_count += 1;
        let mut rejection_reasons = Vec::new();

        if let Some(pool) = remote.pool.as_ref()
            && !candidate.pools.iter().any(|value| value == pool)
        {
            rejection_reasons.push(RemoteCandidateRejection::PoolMismatch {
                required: pool.clone(),
                available: candidate.pools.clone(),
            });
        }

        let missing_tags = remote
            .required_tags
            .iter()
            .filter(|tag| !candidate.tags.iter().any(|value| value == *tag))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_tags.is_empty() {
            rejection_reasons.push(RemoteCandidateRejection::MissingTags {
                missing: missing_tags,
                available: candidate.tags.clone(),
            });
        }

        let missing_capabilities = remote
            .required_capabilities
            .iter()
            .filter(|capability| {
                !candidate
                    .capabilities
                    .iter()
                    .any(|value| value == *capability)
            })
            .cloned()
            .collect::<Vec<_>>();
        if !missing_capabilities.is_empty() {
            rejection_reasons.push(RemoteCandidateRejection::MissingCapabilities {
                missing: missing_capabilities,
                available: candidate.capabilities.clone(),
            });
        }

        let transport_kind =
            StrictRemoteTransportKind::from_inventory_value(candidate.transport.as_str());
        if transport_kind.is_none_or(|kind| !kind.matches_requested(remote.transport_kind)) {
            rejection_reasons.push(RemoteCandidateRejection::TransportMismatch {
                required: remote.transport_kind,
                available: candidate.transport.clone(),
            });
        }

        if rejection_reasons.is_empty() {
            matched_targets.push(StrictRemoteTarget {
                node_id: candidate.node_id.clone(),
                endpoint: candidate.base_url.clone(),
                transport_kind: transport_kind.expect("matching candidate transport kind"),
                bearer_token: candidate.bearer_token.clone(),
                runtime: remote.runtime.clone(),
            });
        }

        enabled_remotes.push(RemoteCandidateDiagnostic {
            node_id: candidate.node_id,
            endpoint: candidate.base_url,
            pools: candidate.pools,
            tags: candidate.tags,
            capabilities: candidate.capabilities,
            transport: candidate.transport,
            rejection_reasons,
        });
    }

    Ok(RemoteTargetSelection {
        configured_remote_count,
        enabled_remote_count,
        enabled_remotes,
        matched_targets,
    })
}

fn inventory_path() -> Result<PathBuf> {
    let root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map_err(|_| anyhow!("failed to resolve config home"))?;
    Ok(root.join("tak").join("remotes.toml"))
}
