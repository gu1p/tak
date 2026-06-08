use serde_json::json;
use tak_core::model::Scope;

use super::wire_types::{
    AcquireLeaseRequest, ClientInfo, NeedRequest, PendingInfo, ReleaseLeaseRequest,
    RenewLeaseRequest, Request, Response, TaskInfo,
};

fn client() -> ClientInfo {
    ClientInfo {
        user: "alice".to_string(),
        pid: 42,
        session_id: "session-1".to_string(),
    }
}

fn task() -> TaskInfo {
    TaskInfo {
        label: "//pkg:task".to_string(),
        attempt: 2,
    }
}

#[test]
fn acquire_request_serializes_to_daemon_wire_shape() {
    let request = Request::Acquire(AcquireLeaseRequest {
        request_id: "request-1".to_string(),
        client: client(),
        task: task(),
        needs: vec![NeedRequest {
            name: "gpu".to_string(),
            scope: Scope::Project,
            scope_key: Some("project-a".to_string()),
            slots: 2.5,
        }],
        ttl_ms: 30_000,
    });

    assert_eq!(
        serde_json::to_value(request).expect("serialize acquire request"),
        json!({
            "type": "AcquireLease",
            "request_id": "request-1",
            "client": {
                "user": "alice",
                "pid": 42,
                "session_id": "session-1"
            },
            "task": {
                "label": "//pkg:task",
                "attempt": 2
            },
            "needs": [{
                "name": "gpu",
                "scope": "project",
                "scope_key": "project-a",
                "slots": 2.5
            }],
            "ttl_ms": 30000
        })
    );
}

#[test]
fn renew_and_release_requests_serialize_to_daemon_wire_shapes() {
    let renew = Request::Renew(RenewLeaseRequest {
        request_id: "renew-1".to_string(),
        lease_id: "lease-1".to_string(),
        ttl_ms: 45_000,
    });
    let release = Request::Release(ReleaseLeaseRequest {
        request_id: "release-1".to_string(),
        lease_id: "lease-1".to_string(),
    });

    assert_eq!(
        serde_json::to_value(renew).expect("serialize renew request"),
        json!({
            "type": "RenewLease",
            "request_id": "renew-1",
            "lease_id": "lease-1",
            "ttl_ms": 45000
        })
    );
    assert_eq!(
        serde_json::to_value(release).expect("serialize release request"),
        json!({
            "type": "ReleaseLease",
            "request_id": "release-1",
            "lease_id": "lease-1"
        })
    );
}

#[test]
fn daemon_responses_deserialize_from_wire_shapes() {
    let granted: Response = serde_json::from_value(json!({
        "type": "LeaseGranted",
        "request_id": "request-1",
        "lease": {
            "lease_id": "lease-1",
            "ttl_ms": 30000,
            "renew_after_ms": 10000
        }
    }))
    .expect("deserialize lease granted");
    let Response::LeaseGranted { request_id, lease } = granted else {
        panic!("expected granted response");
    };
    assert_eq!(request_id, "request-1");
    assert_eq!(lease.lease_id, "lease-1");
    assert_eq!(lease.ttl_ms, 30_000);
    assert_eq!(lease.renew_after_ms, 10_000);

    let pending: Response = serde_json::from_value(json!({
        "type": "LeasePending",
        "request_id": "request-2",
        "pending": {
            "queue_position": 3
        }
    }))
    .expect("deserialize lease pending");
    let Response::LeasePending {
        request_id,
        pending: PendingInfo { queue_position },
    } = pending
    else {
        panic!("expected pending response");
    };
    assert_eq!(request_id, "request-2");
    assert_eq!(queue_position, 3);

    let renewed: Response = serde_json::from_value(json!({
        "type": "LeaseRenewed",
        "request_id": "request-3",
        "ttl_ms": 45000
    }))
    .expect("deserialize lease renewed");
    let Response::LeaseRenewed { request_id, ttl_ms } = renewed else {
        panic!("expected renewed response");
    };
    assert_eq!(request_id, "request-3");
    assert_eq!(ttl_ms, 45_000);

    let released: Response = serde_json::from_value(json!({
        "type": "LeaseReleased",
        "request_id": "request-4"
    }))
    .expect("deserialize lease released");
    let Response::LeaseReleased { request_id } = released else {
        panic!("expected released response");
    };
    assert_eq!(request_id, "request-4");

    let error: Response = serde_json::from_value(json!({
        "type": "Error",
        "request_id": "request-5",
        "message": "capacity unavailable"
    }))
    .expect("deserialize error response");
    let Response::Error {
        request_id,
        message,
    } = error
    else {
        panic!("expected error response");
    };
    assert_eq!(request_id, "request-5");
    assert_eq!(message, "capacity unavailable");
}
