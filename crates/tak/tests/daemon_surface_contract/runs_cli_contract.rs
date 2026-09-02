#![cfg(unix)]

#[path = "runs_cli_contract/attach_interrupt.rs"]
mod attach_interrupt;
#[path = "runs_cli_contract/correlated_rejection.rs"]
mod correlated_rejection;
#[path = "runs_cli_contract/disconnect.rs"]
mod disconnect;
#[path = "runs_cli_contract/docker_delegation.rs"]
mod docker_delegation;
#[path = "runs_cli_contract/docker_dockerfile_delegation.rs"]
mod docker_dockerfile_delegation;
#[path = "runs_cli_contract/docker_remote_delegation.rs"]
mod docker_remote_delegation;
#[path = "runs_cli_contract/exec_delegation.rs"]
mod exec_delegation;
#[path = "runs_cli_contract/exec_missing_daemon.rs"]
mod exec_missing_daemon;
#[path = "runs_cli_contract/exec_missing_environment.rs"]
mod exec_missing_environment;
#[path = "runs_cli_contract/exec_real_daemon.rs"]
mod exec_real_daemon;
#[path = "runs_cli_contract/existing_output_destination.rs"]
mod existing_output_destination;
#[path = "runs_cli_contract/failed_attachment.rs"]
mod failed_attachment;
#[path = "runs_cli_contract/fake_daemon.rs"]
pub(crate) mod fake_daemon;
#[path = "runs_cli_contract/foreground_output_delay.rs"]
mod foreground_output_delay;
#[path = "runs_cli_contract/gapped_attachment.rs"]
mod gapped_attachment;
#[path = "runs_cli_contract/help.rs"]
mod help;
#[path = "runs_cli_contract/huge_output.rs"]
mod huge_output;
#[path = "runs_cli_contract/invalid_run_id.rs"]
mod invalid_run_id;
#[path = "runs_cli_contract/live_attachment_output.rs"]
mod live_attachment_output;
#[path = "runs_cli_contract/long_socket_path.rs"]
mod long_socket_path;
#[path = "runs_cli_contract/missing_daemon.rs"]
mod missing_daemon;
#[path = "runs_cli_contract/outputs_usage.rs"]
mod outputs_usage;
#[path = "runs_cli_contract/real_daemon.rs"]
mod real_daemon;
#[path = "runs_cli_contract/retention_expiry.rs"]
mod retention_expiry;
#[path = "runs_cli_contract/run_id_correlation.rs"]
mod run_id_correlation;
#[path = "runs_cli_contract/successful_commands.rs"]
mod successful_commands;
#[path = "runs_cli_contract/symlink_chain_outputs.rs"]
mod symlink_chain_outputs;
#[path = "runs_cli_contract/terminal_exit_code.rs"]
mod terminal_exit_code;
#[path = "runs_cli_contract/timeout.rs"]
mod timeout;
#[path = "runs_cli_contract/unsafe_outputs.rs"]
mod unsafe_outputs;
#[path = "runs_cli_contract/untrusted_responses.rs"]
mod untrusted_responses;
#[path = "runs_cli_contract/v1_no_retry.rs"]
mod v1_no_retry;
#[path = "runs_cli_contract/v2_routing.rs"]
mod v2_routing;
