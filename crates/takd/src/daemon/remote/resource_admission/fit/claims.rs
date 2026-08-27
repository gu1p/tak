use std::time::Duration;

use crate::daemon::remote::resource_envelope::ELASTIC_STARTUP_DURATION;
use crate::daemon::remote::tak_container_usage::{TakContainerUsageSnapshot, TakTaskUsageSnapshot};

use super::capacity::{add, max_capacity, zero};
use super::{BYTES_PER_MB, ResourceAdmissionState, ResourceCapacity, authored, request_claim};

pub(super) struct ClaimSummary {
    pub(super) reserved: ResourceCapacity,
    pub(super) pending_startup: ResourceCapacity,
    pub(super) actual: ResourceCapacity,
    pub(super) total: ResourceCapacity,
}

pub(super) fn claim_summary(state: &ResourceAdmissionState) -> ClaimSummary {
    let usage = state.tak_container_usage.latest();
    let actual = aggregate_usage(&usage);
    let (reserved, pending_startup) = scheduled_totals(state, &usage);
    let total = if usage.attribution_complete {
        attributed_claim(state, &usage, actual)
    } else {
        add(max_capacity(reserved, actual), pending_startup)
    };
    ClaimSummary {
        reserved,
        pending_startup,
        actual,
        total,
    }
}

fn attributed_claim(
    state: &ResourceAdmissionState,
    usage: &TakContainerUsageSnapshot,
    actual: ResourceCapacity,
) -> ResourceCapacity {
    let mut total = zero();
    for request in state.reservations.values() {
        let (scheduled, _) = scheduled_claim(state, usage, request);
        let measured = usage
            .task_usage
            .get(&request.idempotency_key)
            .map(task_usage)
            .unwrap_or_else(zero);
        total = add(total, max_capacity(scheduled, measured));
    }
    let attributed = usage
        .task_usage
        .iter()
        .fold(zero(), |total, (key, sample)| {
            if state.reservations.contains_key(key) {
                total
            } else {
                add(total, task_usage(sample))
            }
        });
    add(add(total, attributed), residual_usage(actual, usage))
}

fn scheduled_totals(
    state: &ResourceAdmissionState,
    usage: &TakContainerUsageSnapshot,
) -> (ResourceCapacity, ResourceCapacity) {
    state
        .reservations
        .values()
        .fold((zero(), zero()), |(mut reserved, mut startup), request| {
            let (claim, pending) = scheduled_claim(state, usage, request);
            if authored(request) {
                reserved = add(reserved, claim);
            } else if pending {
                startup = add(startup, claim);
            }
            (reserved, startup)
        })
}

fn scheduled_claim(
    state: &ResourceAdmissionState,
    usage: &TakContainerUsageSnapshot,
    request: &super::ResourceRequest,
) -> (ResourceCapacity, bool) {
    let admitted_at = state.admitted_at.get(&request.idempotency_key).copied();
    let elapsed = admitted_at.map(|at| at.elapsed()).unwrap_or(Duration::ZERO);
    let measured = usage
        .task_usage
        .get(&request.idempotency_key)
        .map(task_usage);
    let pending_startup =
        !authored(request) && (elapsed < ELASTIC_STARTUP_DURATION || measured.is_none());
    let claim = request_claim(
        request,
        state.capacity,
        state.elastic_startup,
        elapsed,
        measured,
    );
    (claim, pending_startup)
}

fn aggregate_usage(usage: &TakContainerUsageSnapshot) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: usage.cpu_cores,
        memory_mb: usage.memory_bytes.div_ceil(BYTES_PER_MB),
    }
}

fn task_usage(usage: &TakTaskUsageSnapshot) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: usage.cpu_cores,
        memory_mb: usage.memory_bytes.div_ceil(BYTES_PER_MB),
    }
}

fn residual_usage(
    aggregate: ResourceCapacity,
    usage: &TakContainerUsageSnapshot,
) -> ResourceCapacity {
    let attributed = usage
        .task_usage
        .values()
        .fold(zero(), |total, sample| add(total, task_usage(sample)));
    ResourceCapacity {
        cpu_cores: (aggregate.cpu_cores - attributed.cpu_cores).max(0.0),
        memory_mb: aggregate.memory_mb.saturating_sub(attributed.memory_mb),
    }
}
