use std::thread;
use std::time::Duration;

use takd::SubmitAttemptStore;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

#[path = "remote_resource_admission_behavior/cancel.rs"]
mod cancel;
#[path = "remote_resource_admission_behavior/status.rs"]
mod status;
#[path = "remote_resource_admission_behavior/submit.rs"]
mod submit;

use status::{full_node_limits, status, task_events, wait_for_status, wait_for_task_event};
use submit::submit;

#[tokio::test(flavor = "multi_thread")]
async fn remote_submit_queues_when_detected_resources_are_reserved() {
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
            wait_response_delay: Duration::from_secs(10),
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir)
        .with_skip_exec_root_probe(true);
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let limits = full_node_limits(&context, &store);

    submit(&context, &store, "task-run-1", "sleep 1", limits);
    submit(&context, &store, "task-run-2", "printf queued", limits);

    thread::sleep(Duration::from_millis(100));
    let initial_status = status(&context, &store);
    if initial_status.active_jobs.is_empty() && initial_status.queued_jobs.is_empty() {
        panic!(
            "resource admission did not expose active or queued jobs; task1 events: {:?}; task2 events: {:?}; creates: {:?}",
            task_events(&context, &store, "task-run-1"),
            task_events(&context, &store, "task-run-2"),
            daemon.create_records(),
        );
    }

    let status = if initial_status.active_jobs.len() == 1 && initial_status.queued_jobs.len() == 1 {
        initial_status
    } else {
        wait_for_status(&context, &store, |status| {
            status.active_jobs.len() == 1 && status.queued_jobs.len() == 1
        })
    };
    assert_eq!(status.active_jobs[0].task_run_id, "task-run-1");
    assert_eq!(status.queued_jobs[0].task_run_id, "task-run-2");
    assert_eq!(status.queued_jobs[0].queue_position, 1);

    let events = task_events(&context, &store, "task-run-2");
    assert!(
        events.iter().any(|event| event.kind == "TASK_QUEUED"
            && event
                .message
                .as_deref()
                .is_some_and(|message| message.contains("0 tasks ahead"))),
        "missing queued event: {events:?}"
    );

    let events = wait_for_task_event(&context, &store, "task-run-2", "TASK_STARTED");
    assert!(
        events.iter().any(|event| event.kind == "TASK_QUEUED"),
        "queued event should be preserved after start: {events:?}"
    );
    let status = wait_for_status(&context, &store, |status| status.queued_jobs.is_empty());
    assert!(status.queued_jobs.is_empty());
}
