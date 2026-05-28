use prost::Message;
use tak_proto::{
    FusedTaskMember, NodeInfo, NodePingResponse, RemoteTokenPayload, Step, SubmitTaskRequest,
    SubmittedNeed, decode_remote_token, encode_remote_token,
};

#[test]
fn protobuf_messages_and_tokens_round_trip_as_binary() {
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

    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".to_string(),
        node: Some(NodeInfo {
            node_id: "builder-a".to_string(),
            display_name: "Builder A".to_string(),
            base_url: "http://127.0.0.1:43123".to_string(),
            healthy: true,
            pools: vec!["default".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: "direct".to_string(),
            transport_state: "ready".to_string(),
            transport_detail: String::new(),
        }),
        bearer_token: "secret-token".to_string(),
    })
    .expect("encode token");
    let decoded = decode_remote_token(&token).expect("decode token");
    let node = decoded.node.expect("node");
    assert_eq!(node.node_id, "builder-a");
    assert_eq!(node.transport_state, "ready");
    assert!(node.transport_detail.is_empty());
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
