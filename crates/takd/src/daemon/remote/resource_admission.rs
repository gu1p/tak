use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[path = "resource_admission_cpu_recovery_tests.rs"]
mod cpu_recovery_tests;
mod fit;
mod operations;
mod request;
#[path = "resource_admission_reservation_tests.rs"]
mod reservation_tests;
#[path = "resource_admission_test_support.rs"]
mod test_support;

use super::resource_envelope::ResourceEnvelope;

pub(crate) use request::{ResourceRequest, proto_resource_limits};

use super::tak_container_usage::SharedTakContainerUsage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ResourceCapacity {
    pub(super) cpu_cores: f64,
    pub(super) memory_mb: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HostUsageSample {
    pub(super) non_tak_usage: ResourceCapacity,
    pub(super) available_memory_mb: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ResourceAdmissionSnapshot {
    pub(super) reserved: ResourceCapacity,
    pub(super) pending_startup: ResourceCapacity,
    pub(super) actual: ResourceCapacity,
    pub(super) claimed: ResourceCapacity,
    pub(super) admittable: ResourceCapacity,
    pub(super) host_usage: Option<HostUsageSample>,
    pub(super) execution_capacity: u32,
    pub(super) execution_used: u32,
}

#[derive(Clone)]
pub(crate) struct SharedResourceAdmission {
    inner: Arc<ResourceAdmissionLock>,
}

struct ResourceAdmissionLock {
    state: Mutex<ResourceAdmissionState>,
}

struct ResourceAdmissionState {
    capacity: ResourceCapacity,
    total_capacity: ResourceCapacity,
    execution_capacity: u32,
    margin: ResourceCapacity,
    reservations: BTreeMap<String, ResourceRequest>,
    admitted_at: BTreeMap<String, Instant>,
    /// Cumulative reservations may exceed raw capacity by this factor (>=1):
    /// admission is intentionally tolerant and the memory-pressure controller is
    /// the runtime backstop. Never relaxes `fits_total_capacity`.
    oversubscribe_x: u64,
    tak_container_usage: SharedTakContainerUsage,
    elastic_startup: ResourceCapacity,
    host_usage: Option<HostUsageSample>,
    /// When the controller is in its emergency band it sets this; new starts are
    /// then queued (never admitted) until it clears. Does not evict running work.
    held: bool,
}

impl SharedResourceAdmission {
    pub(super) fn new_with_resource_envelope(
        tak_container_usage: SharedTakContainerUsage,
        envelope: ResourceEnvelope,
        oversubscribe_x: u64,
        elastic_startup: ResourceCapacity,
        host_usage: Option<HostUsageSample>,
    ) -> Self {
        Self {
            inner: Arc::new(ResourceAdmissionLock {
                state: Mutex::new(ResourceAdmissionState {
                    capacity: envelope.workload,
                    total_capacity: envelope.total,
                    execution_capacity: execution_capacity(envelope.total.cpu_cores),
                    margin: envelope.margin,
                    reservations: BTreeMap::new(),
                    admitted_at: BTreeMap::new(),
                    oversubscribe_x: oversubscribe_x.max(1),
                    tak_container_usage,
                    elastic_startup,
                    host_usage,
                    held: false,
                }),
            }),
        }
    }
}

fn execution_capacity(cpu_cores: f64) -> u32 {
    if !cpu_cores.is_finite() || cpu_cores < 1.0 {
        return 1;
    }
    cpu_cores.floor().min(u32::MAX as f64) as u32
}
