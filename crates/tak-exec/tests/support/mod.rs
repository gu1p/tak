#![allow(unused_imports)]

pub mod env;
pub mod fake_docker;
pub mod fake_docker_daemon;
pub mod http;
pub mod inventory;
pub mod lease_order_case;
pub mod lease_order_fused;
pub mod lease_order_need;
pub mod local_tor_broker;
pub mod nonzero_wait_docker_daemon;
pub mod output_spec;
pub mod remote_mismatch;
pub mod remote_progress_wait;
pub mod remote_runtime_spec;
pub mod remote_worker_runtime;
pub mod retryable_tor_daemon;
pub mod servers;
pub mod status_observer;
pub mod task_spec;
pub mod workspace_task_spec;

pub use env::{EnvGuard, LockedEnvGuard, env_lock};
pub use fake_docker::install_fake_docker;
pub use fake_docker_daemon::FakeDockerDaemon;
pub use inventory::{RemoteInventoryRecord, write_remote_inventory};
pub use lease_order_case::{
    RemoteLeaseCase, remote_lease_case, remote_lease_case_with_slow_result,
    remote_lease_case_with_submit_failure,
};
pub use lease_order_fused::fused_remote_cascade_spec;
pub use lease_order_need::add_ui_lock_need;
pub use local_tor_broker::LocalTorBroker;
pub use nonzero_wait_docker_daemon::NonzeroWaitDockerDaemon;
pub use output_spec::{workspace_output_glob, workspace_output_path};
pub use remote_mismatch::{
    prepare_workspace, write_disabled_remote, write_enabled_remote_mismatches,
};
pub use remote_runtime_spec::alpine_spec;
pub use remote_worker_runtime::{
    CollectingObserver, configure_fake_docker_env, configure_real_docker_env, worker_spec,
};
pub use retryable_tor_daemon::RetryableTorDaemon;
pub use servers::{
    AuthRejectingSubmitServer, DelayedEventsServer, EventPollPlan, NonTerminalEventsServer,
    RecordingEvents, RecordingLeaseConfig, RecordingLeaseServer, RecordingRemoteServer,
    RunningTakdServer, ScriptedEventsServer, UploadBeginAuthRejectingServer,
};
pub use status_observer::CollectingStatusObserver;
pub use task_spec::{
    remote_builder_spec, remote_task_spec, remote_task_spec_with_context,
    remote_task_spec_with_context_and_outputs, remote_task_spec_with_outputs, shell_step,
};
