//! Tak daemon protocol and lease coordination engine.
//!
//! The daemon serves NDJSON requests over a Unix socket and coordinates machine-wide
//! limiter leases with optional SQLite-backed persistence and history.

#[allow(dead_code)]
const _TOR_HIDDEN_SERVICE_CONTRACT_MARKER: &str = "arti_client::TorClient launch_onion_service(";

pub mod daemon;

pub use daemon::lease::{
    AcquireLeaseResponse, LeaseManager, SharedLeaseManager, new_shared_manager,
    new_shared_manager_with_db,
};
pub use daemon::protocol::{
    AcquireLeaseRequest, ClientInfo, LeaseInfo, LimiterUsage, NeedRequest, PendingInfo,
    ReleaseLeaseRequest, RenewLeaseRequest, Request, Response, RunTasksRequest, StatusRequest,
    StatusSnapshot, TaskInfo, ensure_valid_request, run_server,
};
pub use daemon::remote::{
    RemoteV1Response, SubmitAttemptStore, SubmitRegistration, build_submit_idempotency_key,
    handle_remote_v1_request, run_remote_v1_http_server, run_remote_v1_tor_hidden_service,
};
pub use daemon::runtime::{default_socket_path, default_state_db_path, run_daemon};
pub use daemon::transport::{
    ArtiSettings, ContainerEngine, ContainerEngineProbe, HostPlatform,
    TorHiddenServiceRuntimeConfig, TorTransportConfig, normalize_tor_transport_config,
    select_container_engine, select_container_engine_with_probe, validate_tor_transport_config,
};
