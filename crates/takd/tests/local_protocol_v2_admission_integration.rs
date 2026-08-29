use tak_core::model::Scope;
use takd::{Request, new_shared_manager_with_db};

use crate::support::protocol::acquire_request;
use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test(flavor = "multi_thread")]
async fn v2_admission_rejects_hybrids_before_legacy_state_can_change() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manager = new_shared_manager_with_db(temp.path().join("takd.sqlite")).expect("manager");
    manager
        .lock()
        .expect("manager lock")
        .set_capacity("cpu", Scope::Machine, None, 1.0);
    let mut daemon = RawLocalProtocol::start_with_manager(manager.clone()).await;
    let legacy = serde_json::to_string(&Request::AcquireLease(acquire_request("hybrid")))
        .expect("encode legacy request");
    let hybrid = legacy.replacen('{', r#"{"protocol_version":2,"#, 1);

    let response = daemon.exchange(&hybrid).await;

    assert!(response.contains(r#""code":"protocol_request_invalid""#));
    let status = manager.lock().expect("manager lock").status();
    assert_eq!(status.active_leases, 0);
    assert_eq!(status.pending_requests, 0);
}
