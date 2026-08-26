use prost::Message;
use tak_proto::{
    FusedTaskMember, GetTaskResultResponse, NodePingResponse, Step, SubmitTaskRequest,
    SubmittedNeed,
};

#[test]
fn protobuf_messages_round_trip_as_binary() {
    let request = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: vec![1, 2, 3],
        steps: vec![Step::default()],
        timeout_s: Some(30),
        runtime: None,
        task_label: "//apps/web:build".to_string(),
        needs: vec![SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 2.0,
        }],
        outputs: Vec::new(),
        session: None,
        origin: None,
        runtime_source: None,
        command: None,
        fused_members: vec![FusedTaskMember {
            task_label: "//apps/web:lint".to_string(),
            steps: Vec::new(),
            timeout_s: None,
            retry: None,
            execution_label: Some("build.lint".to_string()),
        }],
        execution_label: Some("build".to_string()),
        workspace_upload: None,
    };
    let encoded = request.encode_to_vec();
    let decoded = SubmitTaskRequest::decode(encoded.as_slice()).expect("decode request");
    assert_eq!(decoded.task_run_id, "task-run-1");
    assert_eq!(decoded.workspace_zip, vec![1, 2, 3]);
    assert_eq!(decoded.task_label, "//apps/web:build");
    assert_eq!(decoded.execution_label.as_deref(), Some("build"));
    assert_eq!(
        decoded.fused_members[0].execution_label.as_deref(),
        Some("build.lint")
    );
    assert_eq!(decoded.needs.len(), 1);
}

#[test]
fn legacy_result_without_failure_kind_decodes_compatibly() {
    let legacy_wire = vec![0x08, 0x00, 0x10, 0x89, 0x01];
    let decoded = GetTaskResultResponse::decode(legacy_wire.as_slice()).expect("legacy result");

    assert!(!decoded.success);
    assert_eq!(decoded.exit_code, Some(137));
    assert_eq!(decoded.failure_kind, None);
}

#[test]
fn node_ping_response_round_trips_as_binary() {
    let response = NodePingResponse {
        node_id: "builder-a".to_string(),
        protocol_version: "v1".to_string(),
        health: "healthy".to_string(),
        active_job_count: 2,
        queue_depth: 1,
        resource_summary: "cpu=4 memory=8192MiB".to_string(),
    };

    let encoded = response.encode_to_vec();
    let decoded = NodePingResponse::decode(encoded.as_slice()).expect("decode ping");

    assert_eq!(decoded.node_id, "builder-a");
    assert_eq!(decoded.protocol_version, "v1");
    assert_eq!(decoded.health, "healthy");
    assert_eq!(decoded.active_job_count, 2);
    assert_eq!(decoded.queue_depth, 1);
    assert_eq!(decoded.resource_summary, "cpu=4 memory=8192MiB");
}
