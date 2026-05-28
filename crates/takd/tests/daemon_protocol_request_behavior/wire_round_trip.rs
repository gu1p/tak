use takd::{
    CancelTaskRequest, GetOutputRangeRequest, GetTaskResultRequest, PlaceRemoteRequest, Request,
    StreamTaskEventsRequest,
};

#[test]
fn daemon_lifecycle_requests_round_trip_through_wire_contract() {
    let requests = vec![
        Request::PlaceRemote(PlaceRemoteRequest {
            request_id: "place".into(),
            requirements: Default::default(),
            task_run_id: "task-1".into(),
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
