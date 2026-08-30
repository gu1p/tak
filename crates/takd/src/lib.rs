//! Tak execution agent and shared coordination internals.
//!
//! `takd` owns the standalone remote worker HTTP service plus the reusable
//! limiter/submit-store machinery used by agent-facing tests.

#![recursion_limit = "256"]

extern crate self as takd;

#[allow(dead_code)]
const _TOR_HIDDEN_SERVICE_CONTRACT_MARKER: &str = "arti_client::TorClient launch_onion_service(";

pub mod agent;
mod auto_update;
pub mod daemon;
pub mod log_tail;
pub mod service;
#[cfg(test)]
mod test_env;

pub use daemon::lease::{
    AcquireLeaseResponse, LeaseManager, SharedLeaseManager, new_shared_manager_with_db,
};
pub use daemon::peer_manager::{
    LocalNodeIdentity, PeerEligibility, PeerManager, PeerPlacementSelection, PeerSnapshot,
    PeerState, PlacementFailure,
};
pub use daemon::protocol::{
    AcquireLeaseRequest, CancelTaskRequest, ClientInfo, ForwardRemoteHttpRequest,
    GetOutputRangeRequest, GetTaskResultRequest, LeaseInfo, LimiterUsage, NeedRequest,
    PeersEligibleRequest, PeersListRequest, PendingInfo, PlaceRemoteRequest, ReleaseLeaseRequest,
    RemoteResponseHeader, RenewLeaseRequest, Request, Response, StatusRequest, StatusSnapshot,
    StreamTaskEventsRequest, TaskInfo, TorBroker, ensure_valid_request,
    run_server_with_broker_and_peers, run_server_with_broker_peers_and_run_store,
};
pub use daemon::remote::{
    ActiveSubmitAttempt, RemoteImageCacheRuntimeConfig, RemoteNodeContext, RemoteRuntimeConfig,
    RemoteV1Response, SubmitAttemptStore, SubmitRegistration, build_submit_idempotency_key,
    run_remote_v1_http_server,
};
pub use daemon::run_store::{RunStore, SubmitRunResult, UploadProgress};
pub use daemon::runtime::default_socket_path;
pub use daemon::scheduler::{
    AttemptCompletion, DispatchCommand, ResultAcceptance, SchedulerNode, UnknownOutcomeResolution,
};
pub use daemon::transport::{
    ArtiSettings, ContainerEngine, ContainerEngineProbe, HostPlatform,
    TorHiddenServiceRuntimeConfig, TorTransportConfig, select_container_engine,
    select_container_engine_with_probe,
};
pub use service::serve_agent;
