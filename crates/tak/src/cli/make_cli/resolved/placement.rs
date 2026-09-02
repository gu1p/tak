use std::collections::BTreeMap;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::model::{RemoteSelectionSpec, RemoteTransportKind, TaskExecutionSpec};
use tak_core::v2::{
    PlacementCandidate, PlacementKind, PlacementPolicy, RemoteRequirements, RemoteSelection,
};

pub(super) struct Resolution {
    pub(super) policy: PlacementPolicy,
    pub(super) candidates: Vec<PlacementCandidate>,
}

pub(super) async fn resolve(
    execution: &TaskExecutionSpec,
    cache: &mut BTreeMap<RemoteRequirements, Vec<PlacementCandidate>>,
) -> Result<Resolution> {
    let TaskExecutionSpec::RemoteOnly(remote) = execution else {
        if matches!(execution, TaskExecutionSpec::LocalOnly(_)) {
            return Ok(local());
        }
        bail!("synthetic Make task has unsupported execution policy")
    };
    let requirements = RemoteRequirements {
        pool: remote.pool.clone(),
        required_tags: remote.required_tags.clone(),
        required_capabilities: remote.required_capabilities.clone(),
        transport: transport(remote.transport_kind),
    };
    let selection = selection(remote.selection)?;
    if !cache.contains_key(&requirements) {
        let candidates = crate::cli::daemon_run::remote_candidates(requirements.clone()).await?;
        validate_candidates(&requirements, &candidates)?;
        cache.insert(requirements.clone(), candidates);
    }
    Ok(Resolution {
        policy: PlacementPolicy {
            policy_id: policy_id(&requirements, selection),
            selection,
        },
        candidates: cache[&requirements].clone(),
    })
}

fn local() -> Resolution {
    Resolution {
        policy: PlacementPolicy {
            policy_id: "make-local".into(),
            selection: RemoteSelection::Sequential,
        },
        candidates: vec![PlacementCandidate {
            node_id: "local".into(),
            kind: PlacementKind::Local,
            transport: None,
            reason: "local execution through takd".into(),
            tier: 0,
            requirements: None,
        }],
    }
}

fn transport(value: RemoteTransportKind) -> Option<String> {
    match value {
        RemoteTransportKind::Any => None,
        RemoteTransportKind::Direct => Some("direct".into()),
        RemoteTransportKind::Tor => Some("tor".into()),
    }
}

fn selection(value: RemoteSelectionSpec) -> Result<RemoteSelection> {
    match value {
        RemoteSelectionSpec::Balanced => Ok(RemoteSelection::Balanced),
        RemoteSelectionSpec::Sequential => Ok(RemoteSelection::Sequential),
        RemoteSelectionSpec::RoundRobin => Ok(RemoteSelection::RoundRobin),
    }
}

fn validate_candidates(
    requirements: &RemoteRequirements,
    candidates: &[PlacementCandidate],
) -> Result<()> {
    if candidates.is_empty() {
        bail!("no connected protocol-v2 worker matches the Make execution requirements");
    }
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
        bail!("local takd returned a Make candidate outside the requested transport");
    }
    Ok(())
}

fn policy_id(requirements: &RemoteRequirements, selection: RemoteSelection) -> String {
    let encoded = serde_json::to_vec(&(requirements, selection.as_str()))
        .expect("Make placement policy serializes");
    let digest = format!("{:x}", Sha256::digest(encoded));
    format!("make-{}-{}", selection.as_str(), &digest[..16])
}

#[cfg(test)]
mod tests;
