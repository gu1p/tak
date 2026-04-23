use super::types::{
    AcquireLeaseRequest, ClientInfo, NeedRequest, ReleaseLeaseRequest, RenewLeaseRequest, Request,
    StatusRequest, TaskInfo,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RequestEnvelope {
    #[serde(rename = "type")]
    request_type: RequestType,
    request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client: Option<ClientInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    task: Option<TaskInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    needs: Option<Vec<NeedRequest>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ttl_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lease_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RequestType {
    AcquireLease,
    RenewLease,
    ReleaseLease,
    Status,
}

impl TryFrom<RequestEnvelope> for Request {
    type Error = String;

    fn try_from(value: RequestEnvelope) -> std::result::Result<Self, Self::Error> {
        match value.request_type {
            RequestType::AcquireLease => Ok(Self::AcquireLease(AcquireLeaseRequest {
                request_id: value.request_id,
                client: value
                    .client
                    .ok_or_else(|| "AcquireLease requires client".to_string())?,
                task: value
                    .task
                    .ok_or_else(|| "AcquireLease requires task".to_string())?,
                needs: value
                    .needs
                    .ok_or_else(|| "AcquireLease requires needs".to_string())?,
                ttl_ms: value
                    .ttl_ms
                    .ok_or_else(|| "AcquireLease requires ttl_ms".to_string())?,
            })),
            RequestType::RenewLease => Ok(Self::RenewLease(RenewLeaseRequest {
                request_id: value.request_id,
                lease_id: value
                    .lease_id
                    .ok_or_else(|| "RenewLease requires lease_id".to_string())?,
                ttl_ms: value
                    .ttl_ms
                    .ok_or_else(|| "RenewLease requires ttl_ms".to_string())?,
            })),
            RequestType::ReleaseLease => Ok(Self::ReleaseLease(ReleaseLeaseRequest {
                request_id: value.request_id,
                lease_id: value
                    .lease_id
                    .ok_or_else(|| "ReleaseLease requires lease_id".to_string())?,
            })),
            RequestType::Status => Ok(Self::Status(StatusRequest {
                request_id: value.request_id,
            })),
        }
    }
}

impl From<Request> for RequestEnvelope {
    fn from(value: Request) -> Self {
        match value {
            Request::AcquireLease(payload) => Self {
                request_type: RequestType::AcquireLease,
                request_id: payload.request_id,
                client: Some(payload.client),
                task: Some(payload.task),
                needs: Some(payload.needs),
                ttl_ms: Some(payload.ttl_ms),
                lease_id: None,
            },
            Request::RenewLease(payload) => Self {
                request_type: RequestType::RenewLease,
                request_id: payload.request_id,
                client: None,
                task: None,
                needs: None,
                ttl_ms: Some(payload.ttl_ms),
                lease_id: Some(payload.lease_id),
            },
            Request::ReleaseLease(payload) => Self {
                request_type: RequestType::ReleaseLease,
                request_id: payload.request_id,
                client: None,
                task: None,
                needs: None,
                ttl_ms: None,
                lease_id: Some(payload.lease_id),
            },
            Request::Status(payload) => Self {
                request_type: RequestType::Status,
                request_id: payload.request_id,
                client: None,
                task: None,
                needs: None,
                ttl_ms: None,
                lease_id: None,
            },
        }
    }
}
