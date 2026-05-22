#![cfg(test)]

use tak_core::model::ContainerResourceLimitsSpec;

use super::usage::ResourceUsageSnapshot;
use super::{
    ResourceAdmissionLock, ResourceAdmissionState, ResourceCapacity, ResourceRequest,
    SharedResourceAdmission,
};

impl SharedResourceAdmission {
    pub(super) fn new_for_tests(capacity: ResourceCapacity, usage: ResourceUsageSnapshot) -> Self {
        Self {
            inner: std::sync::Arc::new(ResourceAdmissionLock {
                state: std::sync::Mutex::new(ResourceAdmissionState {
                    capacity,
                    usage_source: super::ResourceUsageSource::Fixed(usage),
                    reservations: Default::default(),
                    queue: Default::default(),
                }),
                changed: std::sync::Condvar::new(),
            }),
        }
    }

    pub(crate) fn poison_for_tests(&self) {
        let inner = self.inner.clone();
        let _ = std::thread::spawn(move || {
            let _guard = inner.state.lock().expect("resource admission lock");
            panic!("poison resource admission");
        })
        .join();
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
    }
}
