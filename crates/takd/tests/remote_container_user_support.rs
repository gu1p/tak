use std::fs;

use takd::SubmitAttemptStore;

use crate::support;
use support::env::EnvGuard;
use support::fake_docker_daemon::{CreateRecord, FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use support::wait_for_terminal_events::wait_for_terminal_events;

pub fn setup_container_submit(
    root: &std::path::Path,
    env: &mut EnvGuard,
) -> (
    FakeDockerDaemon,
    takd::RemoteNodeContext,
    SubmitAttemptStore,
) {
    let explicit_root = root.join("explicit-exec-root");
    let tmpdir = root.join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");
    let daemon = FakeDockerDaemon::spawn(
        root,
        FakeDockerConfig {
            visible_roots: vec![explicit_root.clone()],
            image_present: false,
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(root, daemon.socket_path(), env)
        .with_explicit_remote_exec_root(explicit_root)
        .with_temp_dir(tmpdir);
    let context = support::remote_output::test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(root.join("agent.sqlite")).expect("store");
    (daemon, context, store)
}

pub fn submit_successful_container_task(
    context: &takd::RemoteNodeContext,
    store: &SubmitAttemptStore,
    daemon: &FakeDockerDaemon,
    task_run_id: &str,
) -> CreateRecord {
    let ack = submit_container_task(context, store, task_run_id, "true");
    assert!(ack.accepted);
    wait_for_terminal_events(context, store, task_run_id);

    let result = fetch_result(context, store, task_run_id);
    assert!(result.success);
    assert_eq!(result.runtime.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));

    let creates = daemon.create_records();
    assert_eq!(
        creates.len(),
        1,
        "explicit root should skip probe containers"
    );
    assert!(!creates[0].is_probe(), "explicit root should not probe");
    creates[0].clone()
}

pub fn assert_execution_bind_uses_explicit_root(create: &CreateRecord, root: &std::path::Path) {
    let explicit_root = root.join("explicit-exec-root");
    assert!(
        create
            .bind_source()
            .expect("execution bind source")
            .starts_with(&explicit_root),
        "execution bind should use explicit root: {create:?}"
    );
}

#[cfg(unix)]
pub fn current_process_uid_gid() -> String {
    format!("{}:{}", unsafe { libc::geteuid() }, unsafe {
        libc::getegid()
    })
}
