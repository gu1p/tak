#[path = "remote_binary_contract.rs"]
mod binary;
#[path = "remote_cancel_behavior.rs"]
mod cancel;
#[path = "remote_cancel_active_behavior.rs"]
mod cancel_active;
#[path = "remote_cleanup_janitor_behavior.rs"]
mod cleanup_janitor;
#[path = "remote_cleanup_janitor_isolation_behavior.rs"]
mod cleanup_janitor_isolation;
#[path = "remote_cleanup_shared_session_behavior.rs"]
mod cleanup_shared_session;
#[path = "remote_container_cleanup_result_behavior.rs"]
mod container_cleanup_result;
#[path = "remote_container_infrastructure_failure_behavior.rs"]
mod container_infrastructure_failure;
#[path = "remote_container_janitor_behavior.rs"]
mod container_janitor;
#[path = "remote_container_nonzero_exit_behavior.rs"]
mod container_nonzero_exit;
#[path = "remote_container_user_behavior.rs"]
mod container_user;
#[path = "remote_emergency_capacity_behavior.rs"]
mod emergency_capacity;
#[path = "remote_fused_container_oom_retry_behavior.rs"]
mod fused_container_oom_retry;
#[path = "remote_memory_pressure_recovery_behavior.rs"]
mod memory_pressure_recovery;
#[path = "remote_shared_docker_node_isolation_behavior.rs"]
mod shared_docker_node_isolation;
