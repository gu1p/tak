#![allow(dead_code)]

use std::env;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use tak_core::model::{RemoteRuntimeSpec, StepDef, TaskLabel};
use tak_exec::{RemoteWorkerExecutionSpec, TaskOutputChunk, TaskOutputObserver};
use tokio::sync::Notify;

use super::{EnvGuard, install_fake_docker};

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
        attempt: 1,
        steps,
        timeout_s,
        runtime,
        node_id: node_id.to_string(),
        container_user: None,
        image_cache: None,
    }
}

#[derive(Default)]
pub struct CollectingObserver {
    chunks: Mutex<Vec<TaskOutputChunk>>,
    notify: Notify,
}

impl CollectingObserver {
    pub fn snapshot(&self) -> MutexGuard<'_, Vec<TaskOutputChunk>> {
        self.chunks.lock().expect("observer lock")
    }

    pub async fn wait_for_chunks(&self, expected: usize) {
        loop {
            if self.chunks.lock().expect("observer lock").len() >= expected {
                return;
            }
            self.notify.notified().await;
        }
    }
}

impl TaskOutputObserver for CollectingObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> anyhow::Result<()> {
        self.chunks.lock().expect("observer lock").push(chunk);
        self.notify.notify_waiters();
        Ok(())
    }
}
