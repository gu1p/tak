#![allow(clippy::await_holding_lock)]

#[path = "remote_container_user_support.rs"]
mod remote_container_user_support;

use crate::support::env::{EnvGuard, env_lock};
use remote_container_user_support::{
    assert_execution_bind_uses_explicit_root, setup_container_submit,
    submit_successful_container_task,
};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn containerized_remote_tasks_run_with_takd_uid_gid_by_default() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_REMOTE_CONTAINER_USER");
    let temp = tempfile::tempdir().expect("tempdir");
    let (daemon, context, store) = setup_container_submit(temp.path(), &mut env);

    let create = submit_successful_container_task(&context, &store, &daemon, "task-run-user");

    assert_eq!(
        create.user.as_deref(),
        Some(remote_container_user_support::current_process_uid_gid().as_str())
    );
    assert_execution_bind_uses_explicit_root(&create, temp.path());
}

#[tokio::test(flavor = "multi_thread")]
async fn containerized_remote_tasks_can_use_image_default_user() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAKD_REMOTE_CONTAINER_USER", "image");
    let temp = tempfile::tempdir().expect("tempdir");
    let (daemon, context, store) = setup_container_submit(temp.path(), &mut env);

    let create = submit_successful_container_task(&context, &store, &daemon, "task-run-image-user");

    assert_eq!(create.user, None);
    assert_execution_bind_uses_explicit_root(&create, temp.path());
}

#[tokio::test(flavor = "multi_thread")]
async fn containerized_remote_tasks_pass_custom_container_user_override() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAKD_REMOTE_CONTAINER_USER", "0:0");
    let temp = tempfile::tempdir().expect("tempdir");
    let (daemon, context, store) = setup_container_submit(temp.path(), &mut env);

    let create =
        submit_successful_container_task(&context, &store, &daemon, "task-run-custom-user");

    assert_eq!(create.user.as_deref(), Some("0:0"));
    assert_execution_bind_uses_explicit_root(&create, temp.path());
}
