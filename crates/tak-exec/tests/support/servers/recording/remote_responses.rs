use tak_proto::{
    AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse, ErrorResponse,
    FinishWorkspaceUploadResponse, GetTaskResultResponse, NodeInfo, SubmitTaskResponse,
};

pub(super) fn error_response(message: &str) -> ErrorResponse {
    ErrorResponse {
        message: message.into(),
    }
}

pub(super) fn node_info(node_id: &str, port: u16) -> NodeInfo {
    NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
}

pub(super) fn begin_upload_response(upload_id: &str) -> BeginWorkspaceUploadResponse {
    BeginWorkspaceUploadResponse {
        upload_id: upload_id.into(),
        offset: 0,
        complete: false,
    }
}

pub(super) fn append_upload_response(offset: u64) -> AppendWorkspaceUploadResponse {
    AppendWorkspaceUploadResponse {
        offset,
        complete: false,
    }
}

pub(super) fn finish_upload_response(upload_id: &str) -> FinishWorkspaceUploadResponse {
    FinishWorkspaceUploadResponse {
        upload_id: upload_id.into(),
        size_bytes: 0,
        complete: true,
    }
}

pub(super) fn submit_response(idempotency_key: &str) -> SubmitTaskResponse {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: idempotency_key.into(),
        remote_worker: true,
    }
}

pub(super) fn success_result(node_id: &str) -> GetTaskResultResponse {
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
