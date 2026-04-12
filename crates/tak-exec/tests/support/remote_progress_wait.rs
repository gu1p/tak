use tak_proto::GetTaskResultResponse;

pub fn success_result(node_id: &str) -> GetTaskResultResponse {
    GetTaskResultResponse {
        success: true,
        exit_code: Some(0),
        status: "success".into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: node_id.into(),
        transport_kind: "direct".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: None,
    }
}
