use std::collections::VecDeque;

use super::{ResourceAdmissionState, ResourceCapacity, ResourceRequest};

pub(super) fn promote_queued(state: &mut ResourceAdmissionState) {
    loop {
        let Some(next) = state.queue.front().cloned() else {
            return;
        };
        if !can_fit(state, &next) {
            return;
        }
        let next = state.queue.pop_front().expect("queued request");
        state
            .reservations
            .insert(next.idempotency_key.clone(), next);
    }
}

pub(super) fn can_fit(state: &mut ResourceAdmissionState, request: &ResourceRequest) -> bool {
    // Emergency hold: admit nothing new until the controller clears it.
    if state.held {
        return false;
    }
    let used = reserved_totals(state);
    let requested_cpu = request.resource_limits.cpu_cores.unwrap_or(0.0);
    let requested_memory = request.resource_limits.memory_mb.unwrap_or(0);
    // Tolerant admission: oversubscribe capacity by the configured factor. The
    // memory-pressure controller (pause/unpause) is the runtime backstop; we do
    // NOT reject on cumulative reservation pressure here.
    let cpu_budget = state.capacity.cpu_cores * state.oversubscribe_x as f64;
    let mem_budget = state
        .capacity
        .memory_mb
        .saturating_mul(state.oversubscribe_x);
    used.cpu_cores + requested_cpu <= cpu_budget
        && used.memory_mb.saturating_add(requested_memory) <= mem_budget
}

fn reserved_totals(state: &ResourceAdmissionState) -> ResourceCapacity {
    state.reservations.values().fold(
        ResourceCapacity {
            cpu_cores: 0.0,
            memory_mb: 0,
        },
        |mut totals, request| {
            totals.cpu_cores += request.resource_limits.cpu_cores.unwrap_or(0.0);
            totals.memory_mb = totals
                .memory_mb
                .saturating_add(request.resource_limits.memory_mb.unwrap_or(0));
            totals
        },
    )
}

pub(super) fn fits_total_capacity(capacity: &ResourceCapacity, request: &ResourceRequest) -> bool {
    request.resource_limits.cpu_cores.unwrap_or(0.0) <= capacity.cpu_cores
        && request.resource_limits.memory_mb.unwrap_or(0) <= capacity.memory_mb
}

pub(super) fn rejection_reason(capacity: &ResourceCapacity, request: &ResourceRequest) -> String {
    let requested_cpu = request.resource_limits.cpu_cores.unwrap_or(0.0);
    let requested_memory = request.resource_limits.memory_mb.unwrap_or(0);
    format!(
        "requested cpu={requested_cpu:.2}, memory={requested_memory} MB exceeds worker capacity cpu={:.2}, memory={} MB",
        capacity.cpu_cores, capacity.memory_mb
    )
}

pub(super) fn queue_position(
    queue: &VecDeque<ResourceRequest>,
    idempotency_key: &str,
) -> Option<usize> {
    queue
        .iter()
        .position(|request| request.idempotency_key == idempotency_key)
        .map(|index| index + 1)
}
