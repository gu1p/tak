use takd::{
    CancelTaskRequest, GetOutputRangeRequest, GetTaskResultRequest, PeerPlacementSelection,
    PlaceRemoteRequest, Request, Response, StreamTaskEventsRequest,
};

#[test]
fn daemon_lifecycle_requests_round_trip_through_wire_contract() {
    let requests = vec![
        Request::PlaceRemote(PlaceRemoteRequest {
            request_id: "place".into(),
            requirements: Default::default(),
            selection: PeerPlacementSelection::RoundRobin,
            preferred_node_id: None,
            task_run_id: "task-1".into(),
            attempt: 1,
            submit_body: vec![1, 2, 3],
        }),
        Request::StreamTaskEvents(StreamTaskEventsRequest {
            request_id: "events".into(),
            task_handle: "remote:builder-a:task-1".into(),
            after_seq: 7,
        }),
        Request::CancelTask(CancelTaskRequest {
            request_id: "cancel".into(),
            task_handle: "remote:builder-a:task-1".into(),
            attempt: 1,
        }),
        Request::GetTaskResult(GetTaskResultRequest {
            request_id: "result".into(),
            task_handle: "remote:builder-a:task-1".into(),
        }),
        Request::GetOutputRange(GetOutputRangeRequest {
            request_id: "output".into(),
            task_handle: "remote:builder-a:task-1".into(),
            attempt: 1,
            path: "dist/out.txt".into(),
            range: Some("bytes=4-".into()),
        }),
    ];

    for request in requests {
        let encoded = serde_json::to_string(&request).expect("encode request");
        let decoded: Request = serde_json::from_str(&encoded).expect("decode request");
        assert_eq!(format!("{decoded:?}"), format!("{request:?}"));
    }
}

#[test]
fn daemon_error_responses_carry_structured_retry_metadata() {
    let response = Response::Error {
        request_id: "place".into(),
        message: "all Tor peers are unreachable".into(),
        code: Some("all_tor_peers_unreachable".into()),
        retryable: Some(true),
    };

    let encoded = serde_json::to_string(&response).expect("encode response");
    assert!(encoded.contains("\"code\":\"all_tor_peers_unreachable\""));
    assert!(encoded.contains("\"retryable\":true"));

    let decoded: Response = serde_json::from_str(&encoded).expect("decode response");
    match decoded {
        Response::Error {
            code, retryable, ..
        } => {
            assert_eq!(code.as_deref(), Some("all_tor_peers_unreachable"));
            assert_eq!(retryable, Some(true));
        }
        other => panic!("expected error response, got {other:?}"),
    }
}
