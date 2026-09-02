use tak_core::v2::ResourceRequest;

use crate::daemon::scheduler::SchedulerNode;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Usage {
    pub(super) cpu_millis: u64,
    pub(super) memory_bytes: u64,
    pub(super) execution_slots: u64,
    pub(super) unaccepted_cpu_millis: u64,
    pub(super) unaccepted_memory_bytes: u64,
    pub(super) unaccepted_execution_slots: u64,
    pub(super) attempt_count: u64,
}

pub(super) fn has_capacity(
    node: &SchedulerNode,
    reserved: Usage,
    request: ResourceRequest,
) -> bool {
    let Some(cpu) = effective(
        node.cpu_used_millis,
        reserved.cpu_millis,
        reserved.unaccepted_cpu_millis,
    )
    .and_then(|used| used.checked_add(request.cpu_millis)) else {
        return false;
    };
    let Some(memory) = effective(
        node.memory_used_bytes,
        reserved.memory_bytes,
        reserved.unaccepted_memory_bytes,
    )
    .and_then(|used| used.checked_add(request.memory_bytes)) else {
        return false;
    };
    let Some(slots) = effective(
        u64::from(node.execution_used),
        reserved.execution_slots,
        reserved.unaccepted_execution_slots,
    )
    .and_then(|used| used.checked_add(u64::from(request.execution_slots.get()))) else {
        return false;
    };
    cpu <= node.cpu_capacity_millis
        && memory <= node.memory_capacity_bytes
        && slots <= u64::from(node.execution_capacity)
}

fn effective(snapshot: u64, accepted: u64, unaccepted: u64) -> Option<u64> {
    snapshot.max(accepted).checked_add(unaccepted)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PlacementScore {
    pub(super) dominant_pressure: u128,
    queue_pressure: u64,
}

pub(super) fn score_node(
    node: &SchedulerNode,
    reserved: Usage,
    request: ResourceRequest,
    locality: bool,
) -> PlacementScore {
    let current = dominant_pressure(node, reserved, request, true);
    let projected = dominant_pressure(node, reserved, request, false);
    let increment = projected.saturating_sub(current);
    let credit = if locality {
        increment.saturating_sub(1) / 2
    } else {
        0
    };
    PlacementScore {
        dominant_pressure: projected.saturating_sub(credit),
        queue_pressure: u64::from(node.queue_depth).saturating_add(reserved.attempt_count),
    }
}

fn dominant_pressure(
    node: &SchedulerNode,
    reserved: Usage,
    request: ResourceRequest,
    current: bool,
) -> u128 {
    let request_cpu = if current { 0 } else { request.cpu_millis };
    let request_memory = if current { 0 } else { request.memory_bytes };
    let request_slots = if current {
        0
    } else {
        u64::from(request.execution_slots.get())
    };
    [
        ratio(
            node.cpu_used_millis
                .max(reserved.cpu_millis)
                .saturating_add(reserved.unaccepted_cpu_millis)
                .saturating_add(request_cpu),
            node.cpu_capacity_millis,
        ),
        ratio(
            node.memory_used_bytes
                .max(reserved.memory_bytes)
                .saturating_add(reserved.unaccepted_memory_bytes)
                .saturating_add(request_memory),
            node.memory_capacity_bytes,
        ),
        ratio(
            u64::from(node.execution_used)
                .max(reserved.execution_slots)
                .saturating_add(reserved.unaccepted_execution_slots)
                .saturating_add(request_slots),
            u64::from(node.execution_capacity),
        ),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

fn ratio(value: u64, capacity: u64) -> u128 {
    const SCALE: u128 = 1_u128 << 64;
    if value == 0 {
        return 0;
    }
    if capacity == 0 {
        return u128::MAX;
    }
    let numerator = u128::from(value) * SCALE;
    let denominator = u128::from(capacity);
    let quotient = numerator / denominator;
    quotient + u128::from(!numerator.is_multiple_of(denominator))
}
