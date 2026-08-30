#![cfg(unix)]

#[path = "runs_cli_contract/correlated_rejection.rs"]
mod correlated_rejection;
#[path = "runs_cli_contract/disconnect.rs"]
mod disconnect;
#[path = "runs_cli_contract/fake_daemon.rs"]
mod fake_daemon;
#[path = "runs_cli_contract/help.rs"]
mod help;
#[path = "runs_cli_contract/invalid_run_id.rs"]
mod invalid_run_id;
#[path = "runs_cli_contract/missing_daemon.rs"]
mod missing_daemon;
#[path = "runs_cli_contract/outputs_usage.rs"]
mod outputs_usage;
#[path = "runs_cli_contract/real_daemon.rs"]
mod real_daemon;
#[path = "runs_cli_contract/timeout.rs"]
mod timeout;
#[path = "runs_cli_contract/untrusted_responses.rs"]
mod untrusted_responses;
#[path = "runs_cli_contract/v1_no_retry.rs"]
mod v1_no_retry;
#[path = "runs_cli_contract/v2_routing.rs"]
mod v2_routing;
