use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
};
use prost::Message;
use std::time::Duration;
use tak_proto::CancelTaskResponse;
use takd::SubmitAttemptStore;

#[path = "remote_cancel_active_behavior/support.rs"]
mod contract_support;

#[test]
fn cancel_route_terminates_active_remote_worker() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        let daemon = FakeDockerDaemon::spawn(
            temp.path(),
            FakeDockerConfig {
                wait_response_delay: Duration::from_secs(30),
                ..FakeDockerConfig::default()
            },
        );
        let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
            .with_explicit_remote_exec_root(temp.path().join("remote-exec"))
            .with_skip_exec_root_probe(true)
            .build();
        let context = test_context_with_runtime(runtime_config);
        let store =
            SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
        let submit = submit_container_task(&context, &store, "task-run-cancel", "sleep 60");
        assert!(submit.accepted);
        contract_support::wait_until_container_created(&daemon).await;
        let cancel = takd::daemon::remote::handle_remote_v1_request(
            &context,
            &store,
            "POST",
            "/v1/tasks/task-run-cancel/cancel?attempt=1",
            &[],
            None,
        )
        .expect("cancel response");
        assert_eq!(cancel.status_code, 202);
        let cancel =
            CancelTaskResponse::decode(cancel.body.as_slice()).expect("decode cancel response");
        assert!(cancel.cancelled);
        let result = contract_support::wait_for_result(&context, &store, "task-run-cancel").await;
        assert!(!result.success);
        assert_eq!(result.status, "cancelled");
        assert!(result.stderr_tail.unwrap_or_default().contains("cancelled"));
        assert!(!daemon.removed_containers().is_empty());
        assert_eq!(
            contract_support::node_status(&context, &store)
                .active_jobs
                .len(),
            0
        );
    });
}
