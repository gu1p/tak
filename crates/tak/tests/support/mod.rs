#![allow(unused_imports)]

pub mod binary;
pub mod container_runtime;
pub mod coverage_script;
pub mod direct_remote_runtime;
pub mod example_workspace;
pub mod examples_catalog;
pub mod examples_direct_fixture;
pub mod examples_remote_fixture;
pub mod examples_run;
pub mod examples_run_assert;
pub mod examples_run_env;
pub mod examples_surface;
pub mod examples_tor_fixture;
pub mod exec_daemon;
pub mod installer;
pub mod live_direct;
pub mod live_direct_remote;
pub mod live_direct_token;
pub mod live_tor;
pub mod live_tor_remote;
pub mod live_tor_roots;
pub mod local_daemon;
pub mod local_daemon_manager;
#[cfg(unix)]
pub mod make_runtime;
pub mod remote_add;
pub mod remote_cli;
#[cfg(unix)]
pub mod remote_daemon_v2;
pub mod remote_declared_outputs;
pub mod remote_inventory;
pub mod remote_scan;
pub mod root_task_contracts;
pub mod run;
pub mod run_timeout;
pub mod short_daemon_paths;
pub mod takd_binary;
pub mod task_history;
pub mod tasks;
#[cfg(unix)]
pub mod terminal;
pub mod tor_probe_env;
pub mod tor_smoke;
#[cfg(unix)]
pub mod unix_socket_bind_path;
pub mod v2_remote_daemon;

pub use binary::tak_bin;
pub use remote_inventory::{RemoteRecord, write_remote_inventory};
pub use run::{run_tak_expect_failure, run_tak_expect_success, run_tak_output};
pub use tasks::write_tasks;
