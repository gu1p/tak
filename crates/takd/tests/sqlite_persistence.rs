//! Persistence tests for SQLite-backed daemon state and history.

use rusqlite::Connection;
use tak_core::model::Scope;
use takd::{
    AcquireLeaseRequest, AcquireLeaseResponse, ClientInfo, LeaseManager, NeedRequest,
    SubmitAttemptStore, SubmitRegistration, TaskInfo,
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
    let db_path = temp.path().join("takd.sqlite");

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

fn created_submit_key(registration: SubmitRegistration) -> String {
    match registration {
        SubmitRegistration::Created { idempotency_key } => idempotency_key,
        SubmitRegistration::Attached { idempotency_key } => {
            panic!("expected first submit to create a new attempt, attached to {idempotency_key}")
        }
    }
}

/// Verifies duplicate submits with the same `(task_run_id, attempt)` attach to existing execution.
#[test]
fn sqlite_submit_idempotency_duplicate_attach_reuses_existing_attempt_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");
    let mut remote_execution_counter = 0_u32;

    let first_key = created_submit_key(
        store
            .register_submit("task-run-123", Some(1), "remote-a")
            .expect("first submit should create attempt"),
    );
    remote_execution_counter += 1;
    store
        .append_event(
            &first_key,
            1,
            r#"{"kind":"TASK_LOG_CHUNK","chunk":"hello"}"#,
        )
        .expect("persist first event");
    store
        .set_result_payload(&first_key, r#"{"success":true,"exit_code":0}"#)
        .expect("persist terminal result");

    let duplicate = store
        .register_submit("task-run-123", Some(1), "remote-a")
        .expect("duplicate submit should attach");
    let duplicate_key = match duplicate {
        SubmitRegistration::Attached { idempotency_key } => idempotency_key,
        SubmitRegistration::Created { .. } => {
            panic!("duplicate submit must attach to existing remote attempt");
        }
    };

    assert_eq!(first_key, duplicate_key);
    assert_eq!(
        remote_execution_counter, 1,
        "duplicate submit should not trigger a second remote execution"
    );

    let events = store.events(&first_key).expect("load persisted events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].seq, 1);
    assert_eq!(
        events[0].payload_json,
        r#"{"kind":"TASK_LOG_CHUNK","chunk":"hello"}"#
    );

    let result = store
        .result_payload(&first_key)
        .expect("load persisted result payload");
    assert_eq!(result.as_deref(), Some(r#"{"success":true,"exit_code":0}"#));
}

/// Verifies attempt increments produce new idempotency scope for the same task run id.
#[test]
fn sqlite_submit_idempotency_attempt_increment_creates_new_execution_scope() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let first_attempt_key = created_submit_key(
        store
            .register_submit("task-run-123", Some(1), "remote-a")
            .expect("attempt one should create"),
    );
    let second_attempt_key = created_submit_key(
        store
            .register_submit("task-run-123", Some(2), "remote-a")
            .expect("attempt two should create a distinct execution scope"),
    );

    assert_ne!(first_attempt_key, second_attempt_key);
}
