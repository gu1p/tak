#![allow(dead_code)]

use std::collections::BTreeMap;
use std::env;
use std::path::Path;

use tak_core::model::{RemoteRuntimeSpec, StepDef, TaskLabel};
use tak_exec::RemoteWorkerExecutionSpec;

use super::{EnvGuard, install_fake_docker};

mod observer;

pub use observer::CollectingObserver;

pub fn shell_step(script: &str) -> StepDef {
    StepDef::Cmd {
        argv: vec!["sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}

pub fn configure_fake_docker_env(root: &Path, env_guard: &mut EnvGuard) {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);
    env_guard.set(
        "PATH",
        format!(
            "{}:{}",
            bin_root.display(),
            env::var("PATH").unwrap_or_default()
        ),
    );
    env_guard.set("TAK_TEST_HOST_PLATFORM", "other");
}

pub fn configure_real_docker_env(root: &Path, socket_path: &Path, env_guard: &mut EnvGuard) {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);
    env_guard.set(
        "PATH",
        format!(
            "{}:{}",
            bin_root.display(),
            env::var("PATH").unwrap_or_default()
        ),
    );
    env_guard.set("DOCKER_HOST", format!("unix://{}", socket_path.display()));
    env_guard.remove("TAK_TEST_HOST_PLATFORM");
    env_guard.remove("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES");
}

pub fn worker_spec(
    name: &str,
    steps: Vec<StepDef>,
    timeout_s: Option<u64>,
    runtime: Option<RemoteRuntimeSpec>,
    node_id: &str,
) -> RemoteWorkerExecutionSpec {
    RemoteWorkerExecutionSpec {
        task_label: TaskLabel {
            package: "//".into(),
            name: name.into(),
        },
        task_run_id: String::new(),
        attempt: 1,
        steps,
        base_environment: Default::default(),
        clear_environment: false,
        timeout_s,
        runtime,
        node_id: node_id.to_string(),
        container_user: None,
        image_cache: None,
        container_identity: None,
    }
}
