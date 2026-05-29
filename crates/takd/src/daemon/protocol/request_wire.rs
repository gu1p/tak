use super::types::{
    AcquireLeaseRequest, CancelTaskRequest, ClientInfo, ForwardRemoteHttpRequest,
    GetOutputRangeRequest, GetTaskResultRequest, NeedRequest, PeersEligibleRequest,
    PeersListRequest, PlaceRemoteRequest, ReleaseLeaseRequest, RemoteResponseHeader,
    RenewLeaseRequest, Request, StatusRequest, StreamTaskEventsRequest, TaskInfo,
};
use crate::daemon::peer_manager::{PeerEligibility, PeerPlacementSelection};
use serde::{Deserialize, Serialize};

#[path = "request_wire/decode.rs"]
mod decode;
#[path = "request_wire/encode.rs"]
mod encode;
#[path = "request_wire/envelope.rs"]
mod envelope;

pub(super) use envelope::{RequestEnvelope, RequestType};
