use std::path::PathBuf;

use anyhow::{Result, anyhow};
use tak_core::{model::RemoteSpec, remote_inventory::default_remote_inventory_path};

use crate::{
    RemoteCandidateDiagnostic, RemoteCandidateRejection, RemoteTargetSelection, StrictRemoteTarget,
    engine::remote_models::StrictRemoteTransportKind,
};

pub(crate) fn configured_remote_targets(remote: &RemoteSpec) -> Result<RemoteTargetSelection> {
    let path = inventory_path()?;
    let inventory = tak_core::remote_inventory::load_remote_inventory_at(&path)?;
    let configured_remote_count = inventory.remotes.len();
    let mut enabled_remote_count = 0;
    let mut enabled_remotes = Vec::new();
    let mut matched_targets = Vec::new();
    let mut matched_tor_target_count = 0;

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
                !candidate_matches_capability(
                    &candidate.node_id,
                    &candidate.capabilities,
                    capability,
                )
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
        if transport_kind == Some(StrictRemoteTransportKind::Tor) {
            if rejection_reasons.is_empty()
                && matches!(
                    remote.transport_kind,
                    tak_core::model::RemoteTransportKind::Any
                        | tak_core::model::RemoteTransportKind::Tor
                )
            {
                matched_tor_target_count += 1;
            }
            if remote.transport_kind == tak_core::model::RemoteTransportKind::Direct {
                rejection_reasons.push(RemoteCandidateRejection::TransportMismatch {
                    required: remote.transport_kind,
                    available: "tor".to_string(),
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
            continue;
        }
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
                remote_selection: remote.selection,
                required_pool: remote.pool.clone(),
                required_tags: remote.required_tags.clone(),
                required_capabilities: remote.required_capabilities.clone(),
                daemon_task_handle: None,
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
        matched_tor_target_count,
    })
}

fn candidate_matches_capability(
    node_id: &str,
    available_capabilities: &[String],
    required_capability: &str,
) -> bool {
    if let Some(required_node_id) = required_capability.strip_prefix("node:") {
        return required_node_id == node_id;
    }
    available_capabilities
        .iter()
        .any(|value| value == required_capability)
}

fn inventory_path() -> Result<PathBuf> {
    default_remote_inventory_path().map_err(|_| anyhow!("failed to resolve config home"))
}
