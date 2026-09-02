use std::collections::BTreeSet;

use anyhow::{Result, bail};
use tak_core::v2::{
    PlacementCandidate, PlacementKind, PlacementPolicy, RemoteRequirements, RemoteSelection,
};

use super::super::super::{DockerCliSelectors, selectors};

pub(super) async fn resolve(
    selectors: &DockerCliSelectors,
) -> Result<(PlacementPolicy, Vec<PlacementCandidate>)> {
    if selectors.local {
        return Ok(local());
    }
    let candidates =
        super::super::super::super::daemon_run::remote_candidates(requirements(selectors)?).await?;
    if candidates.is_empty() {
        bail!("no connected protocol-v2 worker matches tak docker run");
    }
    Ok((
        PlacementPolicy {
            policy_id: "docker-run-balanced".into(),
            selection: RemoteSelection::Balanced,
        },
        candidates,
    ))
}

fn local() -> (PlacementPolicy, Vec<PlacementCandidate>) {
    (
        PlacementPolicy {
            policy_id: "docker-run-local".into(),
            selection: RemoteSelection::Sequential,
        },
        vec![PlacementCandidate {
            node_id: "local".into(),
            kind: PlacementKind::Local,
            transport: None,
            reason: "local execution through takd".into(),
            tier: 0,
            requirements: None,
        }],
    )
}

fn requirements(selectors: &DockerCliSelectors) -> Result<RemoteRequirements> {
    let mut capabilities = selectors
        .capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if let Some(arch) = selectors.arch.as_deref() {
        capabilities.insert(format!("arch:{}", selectors::normalize_arch(arch)));
    }
    if let Some(os) = selectors.os.as_deref() {
        capabilities.insert(format!("os:{}", selectors::normalize_os(os)));
    }
    if let Some(node) = selectors.node.as_deref() {
        let node = node.trim();
        if node.is_empty() {
            bail!("--node requires a non-empty worker ID");
        }
        capabilities.insert(format!("node:{node}"));
    }
    let transport = match selectors.transport.as_deref() {
        None | Some("any") => None,
        Some("direct" | "tor") => selectors.transport.clone(),
        Some(other) => {
            bail!("unsupported remote transport `{other}`; expected direct, tor, or any")
        }
    };
    Ok(RemoteRequirements {
        pool: selectors.pool.clone(),
        required_tags: selectors.tags.clone(),
        required_capabilities: capabilities.into_iter().collect(),
        transport,
    })
}
