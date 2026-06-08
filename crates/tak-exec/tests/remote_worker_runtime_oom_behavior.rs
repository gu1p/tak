#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_exec::{TaskOutputObserver, execute_remote_worker_steps_with_output};

use crate::support;
use support::{
    CollectingStatusObserver, EnvGuard, NonzeroWaitDockerDaemon, alpine_spec,
    configure_real_docker_env, env_lock,
};

#[tokio::test]
async fn remote_worker_explains_137_with_container_oom_state() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = NonzeroWaitDockerDaemon::spawn_with_inspect_oom(
        temp.path(),
        137,
        Some(false),
        "process killed\n",
    );
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = alpine_spec("remote_runtime_exit_137", "kill -9 $$");
    let observer = Arc::new(CollectingStatusObserver::default());
    let output_observer: Arc<dyn TaskOutputObserver> = observer.clone();

    let result =
        execute_remote_worker_steps_with_output(&workspace_root, &spec, Some(output_observer))
            .await
            .expect("137 container exit should return a task result");

    assert!(!result.success);
    assert_eq!(result.exit_code, Some(137));
    let messages = observer
        .snapshot()
        .iter()
        .map(|event| event.message.clone())
        .collect::<Vec<_>>();
    assert!(
        messages.iter().any(|message| {
            message.contains("exit code 137")
                && message.contains("OOMKilled=false")
                && message.contains("never kills a container for over-using")
                && message.contains("systemd-oomd")
        }),
        "missing 137 diagnostic: {messages:?}"
    );
}
