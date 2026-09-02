use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{
    Execution, PlacementCandidate, PlacementKind, PlacementPolicy, RemoteRequirements,
    RemoteSelection,
};

pub(super) async fn resolve(
    execution: Option<&Execution>,
    socket_path: &Path,
    cache: &mut BTreeMap<RemoteRequirements, Vec<PlacementCandidate>>,
) -> Result<(PlacementPolicy, Vec<PlacementCandidate>)> {
    match execution {
        None => Ok(local("local execution")),
        Some(Execution::LocalOnly { local: authored }) => {
            Ok(local(reason(&authored.reason, "local execution")))
        }
        Some(Execution::RemoteOnly { remote }) => {
            resolve_remote(remote, socket_path, cache, true).await
        }
        Some(Execution::FirstAvailable {
            policy_id,
            placements,
        }) => first_available(policy_id, placements, socket_path, cache).await,
    }
}

async fn resolve_remote(
    remote: &tak_core::v2::RemoteExecution,
    socket_path: &Path,
    cache: &mut BTreeMap<RemoteRequirements, Vec<PlacementCandidate>>,
    require_candidate: bool,
) -> Result<(PlacementPolicy, Vec<PlacementCandidate>)> {
    let requirements = RemoteRequirements::from(remote);
    if !cache.contains_key(&requirements) {
        let candidates =
            super::super::submission::remote_candidates(socket_path, requirements.clone()).await?;
        validate_candidates(&requirements, &candidates)?;
        cache.insert(requirements.clone(), candidates);
    }
    let mut candidates = cache[&requirements].clone();
    if require_candidate && candidates.is_empty() {
        bail!("no connected protocol-v2 worker matches the remote requirements");
    }
    for candidate in &mut candidates {
        candidate.tier = 0;
        if !remote.reason.is_empty() {
            candidate.reason = format!("{}: {}", remote.reason, candidate.reason);
        }
    }
    Ok((
        PlacementPolicy {
            policy_id: policy_id(&requirements, remote.selection),
            selection: remote.selection,
        },
        candidates,
    ))
}

async fn first_available(
    policy_id: &str,
    placements: &[Execution],
    socket_path: &Path,
    cache: &mut BTreeMap<RemoteRequirements, Vec<PlacementCandidate>>,
) -> Result<(PlacementPolicy, Vec<PlacementCandidate>)> {
    let mut candidates = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    for (tier, placement) in placements.iter().enumerate() {
        let mut tier_candidates = match placement {
            Execution::LocalOnly { local: authored } => {
                local(reason(&authored.reason, "local execution")).1
            }
            Execution::RemoteOnly { remote: authored } => {
                resolve_remote(authored, socket_path, cache, false).await?.1
            }
            Execution::FirstAvailable { .. } => {
                bail!("nested Execution.FirstAvailable is not supported")
            }
        };
        tier_candidates.retain(|candidate| seen_nodes.insert(candidate.node_id.clone()));
        let tier = u32::try_from(tier)?;
        for candidate in &mut tier_candidates {
            candidate.tier = tier;
        }
        candidates.extend(tier_candidates);
    }
    if candidates.is_empty() {
        bail!("Execution.FirstAvailable resolved no placement candidates")
    }
    Ok((
        PlacementPolicy {
            policy_id: format!("first-available-{policy_id}"),
            selection: first_selection(placements),
        },
        candidates,
    ))
}

fn local(reason: &str) -> (PlacementPolicy, Vec<PlacementCandidate>) {
    (
        PlacementPolicy {
            policy_id: "local".into(),
            selection: RemoteSelection::Sequential,
        },
        vec![PlacementCandidate {
            node_id: "local".into(),
            kind: PlacementKind::Local,
            transport: None,
            reason: reason.into(),
            tier: 0,
            requirements: None,
        }],
    )
}

fn first_selection(placements: &[Execution]) -> RemoteSelection {
    placements
        .first()
        .map_or(RemoteSelection::Sequential, |placement| match placement {
            Execution::RemoteOnly { remote } => remote.selection,
            Execution::LocalOnly { .. } | Execution::FirstAvailable { .. } => {
                RemoteSelection::Sequential
            }
        })
}

fn reason<'a>(authored: &'a str, fallback: &'a str) -> &'a str {
    if authored.is_empty() {
        fallback
    } else {
        authored
    }
}

fn validate_candidates(
    requirements: &RemoteRequirements,
    candidates: &[PlacementCandidate],
) -> Result<()> {
    if candidates.iter().any(|candidate| {
        candidate.kind != PlacementKind::Remote
            || candidate.transport.as_deref().is_none_or(|transport| {
                !matches!(transport, "direct" | "tor")
                    || requirements
                        .transport
                        .as_deref()
                        .is_some_and(|required| required != transport)
            })
    }) {
        bail!("local takd returned a remote candidate outside the requested transport");
    }
    Ok(())
}

fn policy_id(requirements: &RemoteRequirements, selection: RemoteSelection) -> String {
    let encoded = serde_json::to_vec(&(requirements, selection.as_str()))
        .expect("remote placement policy serializes");
    let digest = format!("{:x}", Sha256::digest(encoded));
    format!("remote-{}-{}", selection.as_str(), &digest[..16])
}
