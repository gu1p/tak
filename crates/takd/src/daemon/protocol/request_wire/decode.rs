use super::*;

impl TryFrom<RequestEnvelope> for Request {
    type Error = String;

    fn try_from(value: RequestEnvelope) -> std::result::Result<Self, Self::Error> {
        match value.request_type {
            RequestType::AcquireLease => Ok(Self::AcquireLease(AcquireLeaseRequest {
                request_id: value.request_id,
                client: required(value.client, "AcquireLease requires client")?,
                task: required(value.task, "AcquireLease requires task")?,
                needs: required(value.needs, "AcquireLease requires needs")?,
                ttl_ms: required(value.ttl_ms, "AcquireLease requires ttl_ms")?,
            })),
            RequestType::RenewLease => Ok(Self::RenewLease(RenewLeaseRequest {
                request_id: value.request_id,
                lease_id: required(value.lease_id, "RenewLease requires lease_id")?,
                ttl_ms: required(value.ttl_ms, "RenewLease requires ttl_ms")?,
            })),
            RequestType::ReleaseLease => Ok(Self::ReleaseLease(ReleaseLeaseRequest {
                request_id: value.request_id,
                lease_id: required(value.lease_id, "ReleaseLease requires lease_id")?,
            })),
            RequestType::Status => Ok(Self::Status(StatusRequest {
                request_id: value.request_id,
            })),
            RequestType::PeersList => Ok(Self::PeersList(PeersListRequest {
                request_id: value.request_id,
            })),
            RequestType::PeersEligible => Ok(Self::PeersEligible(PeersEligibleRequest {
                request_id: value.request_id,
                requirements: value.requirements.unwrap_or_default(),
            })),
            RequestType::PlaceRemote => Ok(Self::PlaceRemote(PlaceRemoteRequest {
                request_id: value.request_id,
                requirements: value.requirements.unwrap_or_default(),
                selection: value.selection.unwrap_or_default(),
                preferred_node_id: value.preferred_node_id,
                task_run_id: required(value.task_run_id, "PlaceRemote requires task_run_id")?,
                attempt: value.attempt.unwrap_or(1),
                submit_body: value.submit_body.unwrap_or_default(),
            })),
            RequestType::ForwardRemoteHttp => {
                Ok(Self::ForwardRemoteHttp(ForwardRemoteHttpRequest {
                    request_id: value.request_id,
                    node_id: required(value.node_id, "ForwardRemoteHttp requires node_id")?,
                    method: required(value.method, "ForwardRemoteHttp requires method")?,
                    path: required(value.path, "ForwardRemoteHttp requires path")?,
                    headers: value.headers.unwrap_or_default(),
                    body: value.body.unwrap_or_default(),
                }))
            }
            RequestType::StreamTaskEvents => Ok(Self::StreamTaskEvents(StreamTaskEventsRequest {
                request_id: value.request_id,
                task_handle: required(value.task_handle, "StreamTaskEvents requires task_handle")?,
                after_seq: value.after_seq.unwrap_or(0),
            })),
            RequestType::CancelTask => Ok(Self::CancelTask(CancelTaskRequest {
                request_id: value.request_id,
                task_handle: required(value.task_handle, "CancelTask requires task_handle")?,
                attempt: required(value.attempt, "CancelTask requires attempt")?,
            })),
            RequestType::GetTaskResult => Ok(Self::GetTaskResult(GetTaskResultRequest {
                request_id: value.request_id,
                task_handle: required(value.task_handle, "GetTaskResult requires task_handle")?,
            })),
            RequestType::GetOutputRange => Ok(Self::GetOutputRange(GetOutputRangeRequest {
                request_id: value.request_id,
                task_handle: required(value.task_handle, "GetOutputRange requires task_handle")?,
                attempt: required(value.attempt, "GetOutputRange requires attempt")?,
                path: required(value.path, "GetOutputRange requires path")?,
                range: value.range,
            })),
        }
    }
}

fn required<T>(value: Option<T>, message: &str) -> std::result::Result<T, String> {
    value.ok_or_else(|| message.to_string())
}
