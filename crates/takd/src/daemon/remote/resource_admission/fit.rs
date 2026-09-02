use std::time::{Duration, Instant};

use super::super::resource_envelope::{ElasticAdmissionClaim, ElasticClaimPolicy};
use super::{ResourceAdmissionSnapshot, ResourceAdmissionState, ResourceCapacity, ResourceRequest};

mod capacity;
mod claims;

use capacity::{effective_capacity, zero};
use claims::claim_summary;

const BYTES_PER_MB: u64 = 1024 * 1024;

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
    let used_slots = execution_used(state);
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
    requested.cpu_cores <= available_cpu
        && requested.memory_mb <= available_memory
        && used_slots.saturating_add(request.execution_slots.get()) <= state.execution_capacity
}

pub(super) fn admission_snapshot(state: &ResourceAdmissionState) -> ResourceAdmissionSnapshot {
    let claims = claim_summary(state);
    let host_usage = state.host_usage;
    let capacity = host_usage
        .map(|sample| effective_capacity(state, sample.non_tak_usage))
        .unwrap_or_else(zero);
    let host_available_memory_mb = host_usage
        .map(|sample| sample.available_memory_mb)
        .unwrap_or(0);
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
        claimed: claims.total,
        admittable,
        host_usage,
        execution_capacity: state.execution_capacity,
        execution_used: execution_used(state),
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

pub(super) fn fits_total_capacity(
    state: &ResourceAdmissionState,
    request: &ResourceRequest,
) -> bool {
    !authored(request)
        || (request.resource_limits.cpu_cores.unwrap_or(0.0) <= state.capacity.cpu_cores
            && request.resource_limits.memory_mb.unwrap_or(0) <= state.capacity.memory_mb
            && request.execution_slots.get() <= state.execution_capacity)
}

fn execution_used(state: &ResourceAdmissionState) -> u32 {
    state.reservations.values().fold(0, |used, request| {
        used.saturating_add(request.execution_slots.get())
    })
}

pub(super) fn authored(request: &ResourceRequest) -> bool {
    request.resource_limits.cpu_cores.is_some() || request.resource_limits.memory_mb.is_some()
}
