//! Tak execution agent and shared coordination internals.
//!
//! `takd` owns the standalone remote worker HTTP service plus the reusable
//! limiter/submit-store machinery used by agent-facing tests.

#[allow(dead_code)]
const _TOR_HIDDEN_SERVICE_CONTRACT_MARKER: &str = "arti_client::TorClient launch_onion_service(";

pub mod agent;
pub mod daemon;
pub mod service;

pub use daemon::lease::{
    AcquireLeaseResponse, LeaseManager, SharedLeaseManager, new_shared_manager,
    new_shared_manager_with_db,
};
pub use daemon::protocol::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    ReleaseLeaseRequest, RenewLeaseRequest, Request, Response, StatusRequest, StatusSnapshot,
    TaskInfo, ensure_valid_request, run_server,
};
pub use daemon::remote::{
    RemoteNodeContext, RemoteV1Response, SubmitAttemptStore, SubmitRegistration,
    build_submit_idempotency_key, handle_remote_v1_request, run_remote_v1_http_server,
    run_remote_v1_tor_hidden_service,
};
pub use daemon::runtime::{default_socket_path, default_state_db_path, run_daemon};
pub use daemon::transport::{
    ArtiSettings, ContainerEngine, ContainerEngineProbe, HostPlatform,
    TorHiddenServiceRuntimeConfig, TorTransportConfig, normalize_tor_transport_config,
    select_container_engine, select_container_engine_with_probe, validate_tor_transport_config,
};
pub use service::serve_agent;
