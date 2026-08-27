use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::super::resource_envelope::{ElasticAdmissionClaim, ElasticClaimPolicy};
use super::{ResourceAdmissionSnapshot, ResourceAdmissionState, ResourceCapacity, ResourceRequest};

mod capacity;
mod claims;

use capacity::{effective_capacity, zero};
use claims::claim_summary;

const BYTES_PER_MB: u64 = 1024 * 1024;

pub(super) fn promote_queued(state: &mut ResourceAdmissionState) -> bool {
    let Some(next) = state.queue.front().cloned() else {
        return false;
    };
    if can_fit(state, &next)
        && let Some(next) = state.queue.pop_front()
    {
        reserve(state, next);
        return true;
    }
    false
}

pub(super) fn reserve(state: &mut ResourceAdmissionState, request: ResourceRequest) {
    state
        .admitted_at
        .insert(request.idempotency_key.clone(), Instant::now());
    state
        .reservations
        .insert(request.idempotency_key.clone(), request);
}

pub(super) fn can_fit(state: &ResourceAdmissionState, request: &ResourceRequest) -> bool {
    let Some(host_usage) = state.host_usage else {
        return false;
    };
    let capacity = effective_capacity(state, host_usage.non_tak_usage);
    if state.held || capacity.cpu_cores <= 0.0 || capacity.memory_mb == 0 {
        return false;
    }
    let used = claim_summary(state).total;
    let requested = request_claim(
        request,
        state.capacity,
        state.elastic_startup,
        Duration::ZERO,
        None,
    );
    let cpu_budget = capacity.cpu_cores * state.oversubscribe_x as f64;
    let memory_budget = capacity.memory_mb.saturating_mul(state.oversubscribe_x);
    let available_cpu = (cpu_budget - used.cpu_cores).max(0.0);
    let available_memory = memory_budget
        .saturating_sub(used.memory_mb)
        .min(host_usage.available_memory_mb);
    requested.cpu_cores <= available_cpu && requested.memory_mb <= available_memory
}

pub(super) fn admission_snapshot(
    state: &ResourceAdmissionState,
    non_tak: ResourceCapacity,
    host_available_memory_mb: u64,
) -> ResourceAdmissionSnapshot {
    let claims = claim_summary(state);
    let capacity = state
        .host_usage
        .map(|_| effective_capacity(state, non_tak))
        .unwrap_or_else(zero);
    let admittable = if state.held {
        zero()
    } else {
        ResourceCapacity {
            cpu_cores: (capacity.cpu_cores - claims.total.cpu_cores).max(0.0),
            memory_mb: capacity
                .memory_mb
                .saturating_sub(claims.total.memory_mb)
                .min(host_available_memory_mb),
        }
    };
    ResourceAdmissionSnapshot {
        reserved: claims.reserved,
        pending_startup: claims.pending_startup,
        actual: claims.actual,
        admittable,
    }
}

pub(super) fn request_claim(
    request: &ResourceRequest,
    capacity: ResourceCapacity,
    elastic_startup: ResourceCapacity,
    elapsed: Duration,
    measured: Option<ResourceCapacity>,
) -> ResourceCapacity {
    if authored(request) {
        return ResourceCapacity {
            cpu_cores: request.resource_limits.cpu_cores.unwrap_or(0.0),
            memory_mb: request.resource_limits.memory_mb.unwrap_or(0),
        };
    }
    match ElasticClaimPolicy::new(elastic_startup).claim_at(elapsed, measured, capacity) {
        ElasticAdmissionClaim::Startup(claim) | ElasticAdmissionClaim::Measured(claim) => claim,
    }
}

pub(super) fn fits_total_capacity(capacity: &ResourceCapacity, request: &ResourceRequest) -> bool {
    !authored(request)
        || (request.resource_limits.cpu_cores.unwrap_or(0.0) <= capacity.cpu_cores
            && request.resource_limits.memory_mb.unwrap_or(0) <= capacity.memory_mb)
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

pub(super) fn authored(request: &ResourceRequest) -> bool {
    request.resource_limits.cpu_cores.is_some() || request.resource_limits.memory_mb.is_some()
}
