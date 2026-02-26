//! Persistence tests for SQLite-backed daemon state and history.

use rusqlite::Connection;
use taskcraft_core::model::Scope;
use taskcraftd::{
    AcquireLeaseRequest, AcquireLeaseResponse, ClientInfo, LeaseManager, NeedRequest, TaskInfo,
};

/// Builds an acquire request fixture for persistence-focused tests.
fn acquire_request(request_id: &str, slots: f64) -> AcquireLeaseRequest {
    AcquireLeaseRequest {
        request_id: request_id.to_string(),
        client: ClientInfo {
            user: "alice".to_string(),
            pid: 4242,
            session_id: "session-1".to_string(),
        },
        task: TaskInfo {
            label: "//apps/web:test_ui".to_string(),
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

/// Verifies active leases and lifecycle events are persisted and restored correctly.
#[test]
fn sqlite_store_persists_active_leases_and_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("taskcraftd.sqlite");

    let lease_id = {
        let mut manager = LeaseManager::with_db_path(db_path.clone()).expect("manager with db");
        manager.set_capacity("cpu", Scope::Machine, None, 4.0);

        let response = manager.acquire(acquire_request("req-acquire", 2.0));
        match response {
            AcquireLeaseResponse::LeaseGranted { lease } => lease.lease_id,
            AcquireLeaseResponse::LeasePending { .. } => panic!("expected lease grant"),
        }
    };

    {
        let conn = Connection::open(&db_path).expect("open db");

        let active_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM active_leases", [], |row| row.get(0))
            .expect("active lease count");
        assert_eq!(active_count, 1);

        let acquire_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM lease_history WHERE event = 'acquire'",
                [],
                |row| row.get(0),
            )
            .expect("acquire history count");
        assert_eq!(acquire_count, 1);
    }

    {
        let mut manager = LeaseManager::with_db_path(db_path.clone()).expect("reload manager");
        manager.set_capacity("cpu", Scope::Machine, None, 4.0);

        let status = manager.status();
        assert_eq!(status.active_leases, 1);
        assert_eq!(status.usage.len(), 1);
        assert_eq!(status.usage[0].used, 2.0);

        manager.release(&lease_id).expect("release lease");
    }

    {
        let conn = Connection::open(&db_path).expect("open db");
        let active_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM active_leases", [], |row| row.get(0))
            .expect("active lease count");
        assert_eq!(active_count, 0);

        let release_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM lease_history WHERE event = 'release'",
                [],
                |row| row.get(0),
            )
            .expect("release history count");
        assert_eq!(release_count, 1);
    }
}
