use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

mod fit;
#[path = "resource_admission_host_usage_tests.rs"]
mod host_usage_tests;
mod operations;
mod request;
#[path = "resource_admission_reservation_tests.rs"]
mod reservation_tests;
#[path = "resource_admission_safety_tests.rs"]
mod safety_tests;
#[path = "resource_admission_test_support.rs"]
mod test_support;
#[path = "resource_admission_tests.rs"]
mod tests;

use super::resource_envelope::ResourceEnvelope;

pub(crate) use request::{ResourceRequest, ResourceRequestInput, proto_resource_limits};

use super::tak_container_usage::SharedTakContainerUsage;

const ADMISSION_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);

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
    pub(super) admittable: ResourceCapacity,
    pub(super) host_usage: Option<HostUsageSample>,
}

#[derive(Clone)]
pub(crate) struct SharedResourceAdmission {
    inner: Arc<ResourceAdmissionLock>,
}

struct ResourceAdmissionLock {
    state: Mutex<ResourceAdmissionState>,
    changed: Condvar,
}

struct ResourceAdmissionState {
    capacity: ResourceCapacity,
    total_capacity: ResourceCapacity,
    margin: ResourceCapacity,
    reservations: BTreeMap<String, ResourceRequest>,
    admitted_at: BTreeMap<String, Instant>,
    queue: VecDeque<ResourceRequest>,
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

#[derive(Debug, Clone)]
pub(crate) enum ResourceAdmissionDecision {
    Admitted,
    Queued { queue_position: usize },
    Rejected { reason: String },
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
                    margin: envelope.margin,
                    reservations: BTreeMap::new(),
                    admitted_at: BTreeMap::new(),
                    queue: VecDeque::new(),
                    oversubscribe_x: oversubscribe_x.max(1),
                    tak_container_usage,
                    elastic_startup,
                    host_usage,
                    held: false,
                }),
                changed: Condvar::new(),
            }),
        }
    }
}
