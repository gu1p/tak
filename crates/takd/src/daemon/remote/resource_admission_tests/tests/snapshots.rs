use super::*;

// Snapshot contracts cover admission accounting exposed to status callers.

#[test]
fn impossible_authored_reservation_rejects_instead_of_queueing() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 4.0,
        memory_mb: 4096,
    });

    let decision = admission
        .admit_or_queue(request("impossible", 5.0, 4096))
        .expect("admission decision");

    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Rejected { .. }
    ));
    assert!(admission.queued_jobs().expect("queued jobs").is_empty());
}

#[test]
fn snapshot_reports_effective_capacity_from_live_usage_and_reservations() {
    let usage = SharedTakContainerUsage::with_snapshot_for_tests(2.0, 3000 * 1024 * 1024);
    let admission = SharedResourceAdmission::new(
        usage,
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 8192,
        },
        1,
    );
    admission
        .admit_or_queue(request("reserved", 1.0, 512))
        .expect("reservation admission");

    let snapshot = admission
        .resource_snapshot(
            ResourceCapacity {
                cpu_cores: 1.0,
                memory_mb: 1024,
            },
            5000,
        )
        .expect("resource snapshot");

    assert_eq!(
        snapshot.reserved,
        ResourceCapacity {
            cpu_cores: 1.0,
            memory_mb: 512,
        }
    );
    assert_eq!(
        snapshot.pending_startup,
        ResourceCapacity {
            cpu_cores: 0.0,
            memory_mb: 0,
        }
    );
    assert_eq!(
        snapshot.actual,
        ResourceCapacity {
            cpu_cores: 2.0,
            memory_mb: 3000,
        }
    );
    assert_eq!(
        snapshot.admittable,
        ResourceCapacity {
            cpu_cores: 5.0,
            memory_mb: 4168,
        }
    );
}

#[test]
fn admission_accounts_for_current_non_tak_use_and_host_availability() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 4.0,
        memory_mb: 4096,
    });
    admission
        .update_host_usage(
            ResourceCapacity {
                cpu_cores: 3.0,
                memory_mb: 3072,
            },
            512,
        )
        .expect("host usage update");

    let decision = admission
        .admit_or_queue(request("next", 2.0, 1024))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Queued { .. }));
}
