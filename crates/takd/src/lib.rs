//! Tak execution agent and shared coordination internals.
//!
//! `takd` owns the standalone remote worker HTTP service plus the reusable
//! limiter/submit-store machinery used by agent-facing tests.

#![recursion_limit = "256"]

extern crate self as takd;

pub mod agent;
mod auto_update;
pub mod daemon;
pub mod log_tail;
pub mod service;
#[cfg(test)]
mod test_env;

pub use daemon::RemoteAttemptTransport;
pub use daemon::attempt_coordinator::{
    AttemptCoordinator, AttemptDispatch, AttemptDriveReport, AttemptObservation, AttemptTransport,
};
pub use daemon::lease::{
    AcquireLeaseResponse, LeaseManager, SharedLeaseManager, new_shared_manager_with_db,
};
pub use daemon::peer_manager::{
    LocalNodeIdentity, PeerEligibility, PeerManager, PeerSnapshot, PeerState, PlacementFailure,
};
pub use daemon::protocol::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    StatusSnapshot, TaskInfo, TorBroker, run_server_with_broker_and_peers,
    run_server_with_broker_peers_and_remote_inventory, run_server_with_broker_peers_and_run_store,
    run_server_with_local_attempt_executable,
    run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown,
    run_server_with_local_attempt_executable_until_shutdown,
};
pub use daemon::remote::{
    ActiveSubmitAttempt, RemoteImageCacheRuntimeConfig, RemoteNodeContext, RemoteRuntimeConfig,
    SubmitAttemptStore, SubmitRegistration, WorkerHttpResponse, build_submit_idempotency_key,
    run_worker_http_server, worker_v2_cancellation_poll_requests_cancel,
};
pub use daemon::run_local_attempt_subprocess;
pub use daemon::run_store::{
    RunAttachmentSnapshot, RunOutputManifest, RunStore, RunStoreMaintenanceConfig,
    RunStoreMaintenanceReport, SubmitRunResult, UploadProgress,
};
pub use daemon::runtime::default_socket_path;
pub use daemon::scheduler::{
    AttemptCompletion, AttemptOutputStream, AttemptRuntimeMetadata, DispatchCommand,
    NodeLossResolution, ResultAcceptance, SchedulerNode, UnknownOutcomeResolution,
};
pub use daemon::transport::{
    ArtiSettings, ContainerEngine, ContainerEngineProbe, HostPlatform,
    TorHiddenServiceRuntimeConfig, TorTransportConfig, select_container_engine,
    select_container_engine_with_probe,
};
pub use daemon::worker_registry::WorkerConnectionTarget;
pub use service::serve_agent;
