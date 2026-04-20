#![allow(unused_imports)]

pub mod env;
pub mod fake_docker;
pub mod fake_docker_daemon;
pub mod http;
pub mod inventory;
pub mod output_spec;
pub mod remote_mismatch;
pub mod remote_progress_wait;
pub mod remote_worker_runtime;
pub mod servers;
pub mod status_observer;
pub mod task_spec;
pub mod workspace_task_spec;

pub use env::{EnvGuard, env_lock};
pub use fake_docker::install_fake_docker;
pub use fake_docker_daemon::FakeDockerDaemon;
pub use inventory::{RemoteInventoryRecord, write_remote_inventory};
pub use output_spec::{workspace_output_glob, workspace_output_path};
pub use remote_mismatch::{
    prepare_workspace, write_disabled_remote, write_enabled_remote_mismatches,
};
pub use remote_worker_runtime::{
    CollectingObserver, configure_fake_docker_env, configure_real_docker_env, worker_spec,
};
pub use servers::{
    AuthRejectingSubmitServer, DelayedEventsServer, EventPollPlan, NonTerminalEventsServer,
    RunningTakdServer, ScriptedEventsServer,
};
pub use status_observer::CollectingStatusObserver;
pub use task_spec::{
    remote_builder_spec, remote_task_spec, remote_task_spec_with_context,
    remote_task_spec_with_context_and_outputs, remote_task_spec_with_outputs, shell_step,
};
