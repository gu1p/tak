//! Behavioral tests for in-memory lease manager grant/pending/release flow.

use tak_core::model::Scope;
use takd::daemon::lease::{AcquireLeaseResponse, LeaseManager};
use takd::daemon::protocol::{AcquireLeaseRequest, ClientInfo, NeedRequest, TaskInfo};

/// Builds a lease acquire request fixture for CPU needs.
fn acquire_request(slots: f64) -> AcquireLeaseRequest {
    AcquireLeaseRequest {
        request_id: "req-1".to_string(),
        client: ClientInfo {
            user: "alice".to_string(),
            pid: 123,
            session_id: "session-1".to_string(),
        },
        task: TaskInfo {
            label: "//apps/web:test".to_string(),
            attempt: 1,
        },
        needs: vec![NeedRequest {
            name: "cpu".to_string(),
            scope: Scope::Machine,
            scope_key: None,
            slots,
        }],
        ttl_ms: 30_000,
    }
}

/// Verifies a lease is granted when requested slots fit available capacity.
#[test]
fn grants_lease_when_capacity_exists() {
    let mut manager = LeaseManager::new();
    manager.set_capacity("cpu", Scope::Machine, None, 4.0);

    let response = manager.acquire(acquire_request(2.0));

    match response {
        AcquireLeaseResponse::LeaseGranted { lease } => {
            assert_eq!(lease.ttl_ms, 30_000);
        }
        AcquireLeaseResponse::LeasePending { .. } => panic!("expected lease grant"),
    }
}

/// Verifies requests become pending when capacity is fully consumed.
#[test]
fn returns_pending_when_capacity_exhausted() {
    let mut manager = LeaseManager::new();
    manager.set_capacity("cpu", Scope::Machine, None, 4.0);

    let first = manager.acquire(acquire_request(4.0));
    assert!(matches!(first, AcquireLeaseResponse::LeaseGranted { .. }));

    let second = manager.acquire(acquire_request(2.0));
    assert!(matches!(
        second,
        AcquireLeaseResponse::LeasePending { pending: _ }
    ));
}

/// Verifies releasing a lease restores capacity for future acquisitions.
#[test]
fn release_frees_capacity_for_future_requests() {
    let mut manager = LeaseManager::new();
    manager.set_capacity("cpu", Scope::Machine, None, 4.0);

    let granted = manager.acquire(acquire_request(4.0));
    let lease_id = match granted {
        AcquireLeaseResponse::LeaseGranted { lease } => lease.lease_id,
        AcquireLeaseResponse::LeasePending { .. } => panic!("expected initial lease grant"),
    };

    manager.release(&lease_id).expect("release should succeed");

    let next = manager.acquire(acquire_request(2.0));
    assert!(matches!(next, AcquireLeaseResponse::LeaseGranted { .. }));
}
