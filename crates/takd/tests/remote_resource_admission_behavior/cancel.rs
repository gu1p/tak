use std::time::Duration;

use prost::Message;
use tak_proto::CancelTaskResponse;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

#[path = "cancel/result.rs"]
mod result;

use super::status::{full_node_limits, wait_for_status};
use super::submit::submit;
use result::wait_for_result;

#[tokio::test(flavor = "multi_thread")]
async fn queued_remote_submit_can_be_cancelled_before_resource_admission() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            image_present: true,
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir)
        .with_skip_exec_root_probe(true);
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let limits = full_node_limits(&context, &store);

    submit(&context, &store, "task-run-active", "sleep 60", limits);
    submit(&context, &store, "task-run-queued", "printf queued", limits);

    wait_for_status(&context, &store, |status| {
        status
            .queued_jobs
            .iter()
            .any(|job| job.task_run_id == "task-run-queued")
    });

    let cancel = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/task-run-queued/cancel?attempt=1",
        None,
    )
    .expect("cancel response");
    assert_eq!(cancel.status_code, 202);
    let cancel = CancelTaskResponse::decode(cancel.body.as_slice()).expect("decode cancel");
    assert!(cancel.cancelled);

    let result = wait_for_result(&context, &store, "task-run-queued");
    assert!(!result.success);
    assert_eq!(result.status, "cancelled");
    assert!(result.stderr_tail.unwrap_or_default().contains("cancelled"));

    let status = wait_for_status(&context, &store, |status| {
        !status
            .queued_jobs
            .iter()
            .any(|job| job.task_run_id == "task-run-queued")
    });
    assert_eq!(status.queued_jobs.len(), 0);
}
