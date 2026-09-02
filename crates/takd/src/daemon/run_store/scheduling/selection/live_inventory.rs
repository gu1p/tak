use tak_core::v2::PlacementCandidate;

use crate::daemon::scheduler::SchedulerNode;

pub(super) fn matches_live_requirements(
    candidate: &PlacementCandidate,
    node: &SchedulerNode,
) -> bool {
    candidate.requirements.as_ref().is_none_or(|requirements| {
        requirements
            .transport
            .as_ref()
            .is_none_or(|value| node.transport.as_ref() == Some(value))
            && requirements
                .pool
                .as_ref()
                .is_none_or(|value| node.pools.contains(value))
            && requirements
                .required_tags
                .iter()
                .all(|value| node.tags.contains(value))
            && requirements
                .required_capabilities
                .iter()
                .all(|value| capability_matches(value, node))
    })
}

fn capability_matches(value: &str, node: &SchedulerNode) -> bool {
    node.capabilities.contains(value)
        || value.strip_prefix("node:").is_some_and(|selector| {
            !selector.is_empty()
                && (selector == node.node_id
                    || node.node_id.starts_with(selector)
                    || selector == tak_core::remote_alias_for_node_id(&node.node_id))
        })
}
