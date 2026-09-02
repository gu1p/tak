use std::num::NonZeroU32;

use tak_core::v2::ResourceRequest;

use super::{Usage, has_capacity, score_node};
use crate::daemon::scheduler::SchedulerNode;

#[test]
fn accepted_origin_usage_already_present_in_snapshot_counts_once() {
    let mut node = SchedulerNode::with_execution_slots("worker-a", 2).with_execution_usage(1);
    node.cpu_capacity_millis = 200;
    node.cpu_used_millis = 100;
    node.memory_capacity_bytes = 200;
    node.memory_used_bytes = 100;
    let accepted = Usage {
        cpu_millis: 100,
        memory_bytes: 100,
        execution_slots: 1,
        attempt_count: 1,
        ..Usage::default()
    };
    let request = ResourceRequest {
        cpu_millis: 100,
        memory_bytes: 100,
        execution_slots: NonZeroU32::MIN,
    };

    assert!(has_capacity(&node, accepted, request));
    assert_eq!(
        score_node(&node, accepted, request, false).dominant_pressure,
        score_node(&node, Usage::default(), request, false).dominant_pressure
    );
}
