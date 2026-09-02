#![cfg(test)]

use std::num::NonZeroU32;

use super::test_support::request;
use super::{ResourceCapacity, SharedResourceAdmission};

#[test]
fn execution_slots_are_hard_capped_and_released_despite_resource_oversubscription() {
    let admission = SharedResourceAdmission::new_for_tests_with_oversubscribe(
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 4096,
        },
        4,
    );
    let mut first = request("first", 1.0, 1);
    first.execution_slots = NonZeroU32::new(8).unwrap();
    let mut second = request("second", 1.0, 1);
    second.execution_slots = NonZeroU32::new(8).unwrap();

    assert!(admission.admit_immediately(first).unwrap());
    assert!(!admission.admit_immediately(second.clone()).unwrap());
    let full = admission.resource_snapshot().unwrap();
    assert_eq!((full.execution_used, full.execution_capacity), (8, 8));
    admission.release("first").unwrap();
    assert_eq!(admission.resource_snapshot().unwrap().execution_used, 0);
    assert!(admission.admit_immediately(second).unwrap());
    admission.release("second").unwrap();
    assert_eq!(admission.resource_snapshot().unwrap().execution_used, 0);
}

#[test]
fn immediate_admission_does_not_enqueue_when_capacity_is_busy() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 2.0,
        memory_mb: 1024,
    });
    let mut first = request("first", 1.0, 1);
    first.execution_slots = NonZeroU32::new(2).unwrap();
    let mut second = request("second", 1.0, 1);
    second.execution_slots = NonZeroU32::new(2).unwrap();

    assert!(admission.admit_immediately(first).unwrap());
    assert!(!admission.admit_immediately(second).unwrap());
    assert_eq!(admission.reserved_jobs().unwrap().len(), 1);
}
