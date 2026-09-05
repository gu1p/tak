use tak_proto::local_daemon::v2::{Response, decode_response};

#[test]
fn legacy_run_details_round_trip_does_not_add_new_v2_fields() {
    let raw = br#"{"protocol_version":2,"type":"RunDetails","request_id":"show","run":{"summary":{"run_id":"run-1","state":"queued","created_at_ms":1,"updated_at_ms":2,"targets":["//:check"],"total_jobs":1,"terminal_jobs":0},"jobs":[{"job_id":"job-0","task_ids":["//:check"],"state":"ready","node_id":null,"attempt":0,"cache":null}]}}"#;
    let response = decode_response(raw, "show").unwrap();
    let encoded = serde_json::to_value(response).unwrap();
    let run = &encoded["run"];

    assert!(run.get("max_parallel_jobs").is_none(), "{encoded}");
    assert!(run["jobs"][0].get("queue").is_none(), "{encoded}");
    assert!(
        run["jobs"][0].get("placement_candidate_node_ids").is_none(),
        "{encoded}"
    );
}

#[test]
fn initial_v2_run_details_still_decode_for_dashboard_attachment() {
    let raw = br#"{"protocol_version":2,"type":"RunDetails","request_id":"show","run":{"summary":{"run_id":"run-1","state":"queued","created_at_ms":1,"updated_at_ms":2,"targets":["//:check"],"total_jobs":1,"terminal_jobs":0},"jobs":[{"job_id":"job-0","task_ids":["//:check"],"state":"ready","node_id":null,"attempt":0,"cache":null}]}}"#;
    let Response::RunDetails { run, .. } = decode_response(raw, "show").unwrap() else {
        panic!("expected run details")
    };

    assert_eq!(run.jobs[0].task_ids, ["//:check"]);
    assert_eq!(run.jobs[0].state, "ready");
    assert_eq!(run.max_parallel_jobs, 0);
    assert_eq!(run.jobs[0].queue, None);
    assert!(run.jobs[0].placement_candidate_node_ids.is_empty());
}

#[test]
fn dashboard_run_details_decode_persisted_scheduler_metadata() {
    let raw = br#"{"protocol_version":2,"type":"RunDetails","request_id":"show","run":{"summary":{"run_id":"run-1","state":"running","created_at_ms":1,"updated_at_ms":2,"targets":["//:check"],"total_jobs":1,"terminal_jobs":0},"max_parallel_jobs":3,"jobs":[{"job_id":"job-0","task_ids":["//:check"],"state":"ready","node_id":null,"attempt":0,"cache":null,"queue":"builds","placement_candidate_node_ids":["worker-a","worker-b"]}]}}"#;
    let Response::RunDetails { run, .. } = decode_response(raw, "show").unwrap() else {
        panic!("expected run details")
    };

    assert_eq!(run.max_parallel_jobs, 3);
    assert_eq!(run.jobs[0].queue.as_deref(), Some("builds"));
    assert_eq!(
        run.jobs[0].placement_candidate_node_ids,
        ["worker-a", "worker-b"]
    );
}

#[test]
fn authored_attempt_is_optional_for_legacy_events_and_round_trips_when_present() {
    let legacy = br#"{"protocol_version":2,"type":"RunEvents","request_id":"legacy","run_id":"run-1","events":[{"seq":1,"kind":"queued","job_id":"job-0","task_ids":["//:check"],"node_id":null,"message":"queued"}],"next_event":1,"state":"queued","terminal":false}"#;
    let legacy = decode_response(legacy, "legacy").unwrap();
    let Response::RunEvents { events, .. } = &legacy else {
        panic!("expected legacy run events")
    };
    assert_eq!(events[0].authored_attempt, None);
    assert!(
        serde_json::to_value(legacy).unwrap()["events"][0]
            .get("authored_attempt")
            .is_none()
    );

    let current = br#"{"protocol_version":2,"type":"RunEvents","request_id":"current","run_id":"run-1","events":[{"seq":2,"kind":"transferring","job_id":"job-0","task_ids":["//:check"],"node_id":"worker-a","message":"job reserved and transferring","authored_attempt":2}],"next_event":2,"state":"running","terminal":false}"#;
    let current = decode_response(current, "current").unwrap();
    let Response::RunEvents { events, .. } = &current else {
        panic!("expected current run events")
    };
    assert_eq!(events[0].authored_attempt, Some(2));
    assert_eq!(
        serde_json::to_value(current).unwrap()["events"][0]["authored_attempt"],
        2
    );
}

#[test]
fn zero_authored_attempt_is_rejected() {
    let malformed = br#"{"protocol_version":2,"type":"RunEvents","request_id":"zero","run_id":"run-1","events":[{"seq":1,"kind":"transferring","job_id":"job-0","task_ids":["//:check"],"node_id":"worker-a","message":"job reserved and transferring","authored_attempt":0}],"next_event":1,"state":"running","terminal":false}"#;

    assert!(decode_response(malformed, "zero").is_err());
}
