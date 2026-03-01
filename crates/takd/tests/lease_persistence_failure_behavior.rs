use tak_core::model::Scope;
use takd::daemon::lease::{AcquireLeaseResponse, LeaseManager};
use takd::daemon::protocol::{AcquireLeaseRequest, ClientInfo, NeedRequest, TaskInfo};

#[test]
fn acquire_does_not_panic_when_sqlite_persistence_is_unavailable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let mut manager = LeaseManager::with_db_path(db_path.clone()).expect("sqlite-backed manager");
    manager.set_capacity("cpu", Scope::Machine, None, 1.0);

    std::fs::remove_file(&db_path).expect("remove sqlite file");
    std::fs::create_dir(&db_path).expect("replace sqlite file with directory");

    let acquired = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        manager.acquire(acquire_request())
    }));
    assert!(
        acquired.is_ok(),
        "acquire must never panic when persistence writes fail"
    );
    assert!(
        matches!(
            acquired.expect("caught response"),
            AcquireLeaseResponse::LeasePending { .. }
        ),
        "failed persistence should keep request pending instead of crashing"
    );

    let snapshot = manager.status();
    assert_eq!(
        snapshot.active_leases, 0,
        "failed persistence should not leave an in-memory active lease"
    );
}

fn acquire_request() -> AcquireLeaseRequest {
    AcquireLeaseRequest {
        request_id: "req-1".to_string(),
        client: ClientInfo {
            user: "user".to_string(),
            pid: 1000,
            session_id: "sess".to_string(),
        },
        task: TaskInfo {
            label: "//pkg:task".to_string(),
            attempt: 1,
        },
        needs: vec![NeedRequest {
            name: "cpu".to_string(),
            scope: Scope::Machine,
            scope_key: None,
            slots: 1.0,
        }],
        ttl_ms: 30_000,
    }
}
