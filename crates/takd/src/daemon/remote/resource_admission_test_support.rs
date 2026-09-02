#![cfg(test)]

use std::num::NonZeroU32;

use tak_core::model::ContainerResourceLimitsSpec;

use super::super::tak_container_usage::SharedTakContainerUsage;
use super::{ResourceCapacity, ResourceRequest, SharedResourceAdmission};

#[path = "resource_admission_test_support/constructors.rs"]
mod constructors;

impl SharedResourceAdmission {
    pub(super) fn new_for_tests(capacity: ResourceCapacity) -> Self {
        Self::new_for_tests_with_oversubscribe(capacity, 1)
    }

    pub(super) fn new_for_tests_with_oversubscribe(
        capacity: ResourceCapacity,
        oversubscribe_x: u64,
    ) -> Self {
        Self::new_with_elastic_startup(
            SharedTakContainerUsage::default(),
            capacity,
            oversubscribe_x,
            ResourceCapacity {
                cpu_cores: 4.0,
                memory_mb: 8 * 1024,
            },
        )
    }
}

pub(super) fn request(id: &str, cpu_cores: f64, memory_mb: u64) -> ResourceRequest {
    ResourceRequest {
        idempotency_key: id.to_string(),
        task_run_id: id.to_string(),
        attempt: 1,
        task_label: "//:check".to_string(),
        queued_at_ms: 1,
        resource_limits: ContainerResourceLimitsSpec {
            cpu_cores: Some(cpu_cores),
            memory_mb: Some(memory_mb),
        },
        runtime: Some("containerized".to_string()),
        origin: Some("task".to_string()),
        runtime_source: Some("image:alpine:3.20".to_string()),
        command: Some("true".to_string()),
        execution_label: None,
        execution_slots: NonZeroU32::MIN,
    }
}
