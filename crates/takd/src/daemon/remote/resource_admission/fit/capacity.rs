use super::super::{ResourceAdmissionState, ResourceCapacity};

pub(super) fn effective_capacity(
    state: &ResourceAdmissionState,
    non_tak: ResourceCapacity,
) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: state.capacity.cpu_cores.min(
            (state.total_capacity.cpu_cores - non_tak.cpu_cores - state.margin.cpu_cores).max(0.0),
        ),
        memory_mb: state.capacity.memory_mb.min(
            state
                .total_capacity
                .memory_mb
                .saturating_sub(non_tak.memory_mb)
                .saturating_sub(state.margin.memory_mb),
        ),
    }
}

pub(super) fn add(left: ResourceCapacity, right: ResourceCapacity) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: left.cpu_cores + right.cpu_cores,
        memory_mb: left.memory_mb.saturating_add(right.memory_mb),
    }
}

pub(super) fn max_capacity(left: ResourceCapacity, right: ResourceCapacity) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: left.cpu_cores.max(right.cpu_cores),
        memory_mb: left.memory_mb.max(right.memory_mb),
    }
}

pub(super) fn zero() -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: 0.0,
        memory_mb: 0,
    }
}
