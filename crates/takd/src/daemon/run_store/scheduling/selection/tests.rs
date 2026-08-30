use std::num::NonZeroU32;

use tak_core::v2::ResourceRequest;

use super::{Usage, has_capacity, score_node};
use crate::daemon::scheduler::SchedulerNode;

#[test]
fn locality_credit_never_lets_a_saturated_node_beat_an_idle_node() {
    let request = ResourceRequest {
        cpu_millis: 0,
        memory_bytes: 0,
        execution_slots: NonZeroU32::MIN,
    };
    let idle = SchedulerNode::with_execution_slots("idle", 10);
    let saturated = SchedulerNode::with_execution_slots("cached", 10).with_execution_usage(9);

    let idle_score = score_node(&idle, Usage::default(), request, false);
    let cached_score = score_node(&saturated, Usage::default(), request, true);

    assert!(idle_score < cached_score);
}

#[test]
fn equal_unit_loads_remain_distinguishable_above_one_billion_slots() {
    let request = ResourceRequest {
        cpu_millis: 0,
        memory_bytes: 0,
        execution_slots: NonZeroU32::MIN,
    };
    let empty = SchedulerNode::with_execution_slots("empty", u32::MAX);
    let used = SchedulerNode::with_execution_slots("used", u32::MAX).with_execution_usage(1);

    assert!(
        score_node(&empty, Usage::default(), request, false)
            < score_node(&used, Usage::default(), request, false)
    );
}

#[test]
fn resource_arithmetic_overflow_is_not_treated_as_capacity() {
    let mut node = SchedulerNode::with_execution_slots("node", 1);
    node.cpu_capacity_millis = u64::MAX;
    node.cpu_used_millis = u64::MAX - 1;
    let request = ResourceRequest {
        cpu_millis: 2,
        memory_bytes: 0,
        execution_slots: NonZeroU32::MIN,
    };

    assert!(!has_capacity(&node, Usage::default(), request));
}

#[test]
fn locality_credit_is_strictly_below_half_the_projected_increment() {
    let request = ResourceRequest {
        cpu_millis: 0,
        memory_bytes: 0,
        execution_slots: NonZeroU32::MIN,
    };
    let node = SchedulerNode::with_execution_slots("node", 10);
    let plain = score_node(&node, Usage::default(), request, false);
    let local = score_node(&node, Usage::default(), request, true);

    assert!(plain.dominant_pressure - local.dominant_pressure < plain.dominant_pressure / 2);
}
