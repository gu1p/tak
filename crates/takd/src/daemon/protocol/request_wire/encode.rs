use super::*;

impl From<Request> for RequestEnvelope {
    fn from(value: Request) -> Self {
        match value {
            Request::AcquireLease(payload) => request_envelope(
                RequestType::AcquireLease,
                payload.request_id,
                Some(payload.client),
                Some(payload.task),
                Some(payload.needs),
                Some(payload.ttl_ms),
            ),
            Request::RenewLease(payload) => lease_envelope(
                RequestType::RenewLease,
                payload.request_id,
                Some(payload.lease_id),
                Some(payload.ttl_ms),
            ),
            Request::ReleaseLease(payload) => lease_envelope(
                RequestType::ReleaseLease,
                payload.request_id,
                Some(payload.lease_id),
                None,
            ),
            Request::Status(payload) => base(RequestType::Status, payload.request_id),
            Request::PeersList(payload) => base(RequestType::PeersList, payload.request_id),
            Request::PeersEligible(payload) => {
                let mut envelope = base(RequestType::PeersEligible, payload.request_id);
                envelope.requirements = Some(payload.requirements);
                envelope
            }
            Request::PlaceRemote(payload) => {
                let mut envelope = base(RequestType::PlaceRemote, payload.request_id);
                envelope.requirements = Some(payload.requirements);
                envelope.selection = Some(payload.selection);
                envelope.preferred_node_id = payload.preferred_node_id;
                envelope.task_run_id = Some(payload.task_run_id);
                envelope.attempt = Some(payload.attempt);
                envelope.submit_body = Some(payload.submit_body);
                envelope
            }
            Request::ForwardRemoteHttp(payload) => forward_remote_envelope(payload),
            Request::StreamTaskEvents(payload) => {
                let mut envelope = base(RequestType::StreamTaskEvents, payload.request_id);
                envelope.task_handle = Some(payload.task_handle);
                envelope.after_seq = Some(payload.after_seq);
                envelope
            }
            Request::CancelTask(payload) => {
                let mut envelope = base(RequestType::CancelTask, payload.request_id);
                envelope.task_handle = Some(payload.task_handle);
                envelope.attempt = Some(payload.attempt);
                envelope
            }
            Request::GetTaskResult(payload) => {
                let mut envelope = base(RequestType::GetTaskResult, payload.request_id);
                envelope.task_handle = Some(payload.task_handle);
                envelope
            }
            Request::GetOutputRange(payload) => output_range_envelope(payload),
        }
    }
}

fn base(request_type: RequestType, request_id: String) -> RequestEnvelope {
    RequestEnvelope {
        request_type,
        request_id,
        client: None,
        task: None,
        needs: None,
        ttl_ms: None,
        lease_id: None,
        requirements: None,
        selection: None,
        task_run_id: None,
        submit_body: None,
        node_id: None,
        preferred_node_id: None,
        method: None,
        path: None,
        headers: None,
        body: None,
        task_handle: None,
        after_seq: None,
        attempt: None,
        range: None,
    }
}

fn request_envelope(
    request_type: RequestType,
    request_id: String,
    client: Option<ClientInfo>,
    task: Option<TaskInfo>,
    needs: Option<Vec<NeedRequest>>,
    ttl_ms: Option<u64>,
) -> RequestEnvelope {
    let mut envelope = base(request_type, request_id);
    envelope.client = client;
    envelope.task = task;
    envelope.needs = needs;
    envelope.ttl_ms = ttl_ms;
    envelope
}

fn lease_envelope(
    request_type: RequestType,
    request_id: String,
    lease_id: Option<String>,
    ttl_ms: Option<u64>,
) -> RequestEnvelope {
    let mut envelope = base(request_type, request_id);
    envelope.lease_id = lease_id;
    envelope.ttl_ms = ttl_ms;
    envelope
}

fn forward_remote_envelope(payload: ForwardRemoteHttpRequest) -> RequestEnvelope {
    let mut envelope = base(RequestType::ForwardRemoteHttp, payload.request_id);
    envelope.node_id = Some(payload.node_id);
    envelope.method = Some(payload.method);
    envelope.path = Some(payload.path);
    envelope.headers = Some(payload.headers);
    envelope.body = Some(payload.body);
    envelope
}

fn output_range_envelope(payload: GetOutputRangeRequest) -> RequestEnvelope {
    let mut envelope = base(RequestType::GetOutputRange, payload.request_id);
    envelope.path = Some(payload.path);
    envelope.task_handle = Some(payload.task_handle);
    envelope.attempt = Some(payload.attempt);
    envelope.range = payload.range;
    envelope
}
