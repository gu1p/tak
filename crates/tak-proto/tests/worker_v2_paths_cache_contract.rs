use tak_core::v2::OutputSelector;
use tak_proto::worker_v2::{
    WorkerWorkspaceReuse, decode_dispatch_request, encode_dispatch_request, payload_digest,
};

use super::worker_v2_attempt_support::{payload, request};

#[test]
fn paths_cache_round_trips_its_session_and_selectors() {
    let mut request = request(payload());
    request.payload.workspace_reuse = WorkerWorkspaceReuse::Paths {
        session_id: "compiler".into(),
        paths: vec![OutputSelector::Path {
            value: ".cache".into(),
        }],
    };
    request.payload_digest = payload_digest(&request.payload).unwrap();

    let decoded = decode_dispatch_request(&encode_dispatch_request(&request).unwrap()).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn paths_cache_rejects_empty_sessions_and_selector_lists() {
    for (session_id, paths) in [("", vec![path("cache")]), ("compiler", vec![])] {
        let mut request = request(payload());
        request.payload.workspace_reuse = WorkerWorkspaceReuse::Paths {
            session_id: session_id.into(),
            paths,
        };
        request.payload_digest = payload_digest(&request.payload).unwrap();
        assert!(encode_dispatch_request(&request).is_err());
    }
}

fn path(value: &str) -> OutputSelector {
    OutputSelector::Path {
        value: value.into(),
    }
}
