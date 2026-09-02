use std::collections::BTreeSet;

use tak_core::v2::{PlacementCandidate, PlacementKind, RemoteRequirements};

use super::matches_live_requirements;
use crate::daemon::scheduler::SchedulerNode;

#[test]
fn candidate_requirements_are_rechecked_against_live_node_inventory() {
    let candidate = PlacementCandidate {
        node_id: "worker-a".into(),
        kind: PlacementKind::Remote,
        transport: Some("direct".into()),
        reason: "resolved while eligible".into(),
        tier: 0,
        requirements: Some(RemoteRequirements {
            pool: Some("build".into()),
            required_tags: vec!["builder".into()],
            required_capabilities: vec!["linux".into()],
            transport: Some("direct".into()),
        }),
    };
    let mut node = SchedulerNode::with_execution_slots("worker-a", 1);
    node.pools = BTreeSet::from(["build".into()]);
    node.tags = BTreeSet::from(["builder".into()]);
    node.capabilities = BTreeSet::from(["linux".into()]);
    assert!(matches_live_requirements(&candidate, &node));

    node.pools.clear();
    assert!(!matches_live_requirements(&candidate, &node));
    node.pools.insert("build".into());
    node.tags.clear();
    assert!(!matches_live_requirements(&candidate, &node));
    node.tags.insert("builder".into());
    node.capabilities.clear();
    assert!(!matches_live_requirements(&candidate, &node));
}
