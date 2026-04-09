#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tak_core::model::{LimiterDef, QueueDef, Scope, WorkspaceSpec};
use takd::{new_shared_manager, run_server};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

pub struct LocalDaemonGuard {
    runtime: Runtime,
    task: JoinHandle<()>,
    socket_path: PathBuf,
}

impl LocalDaemonGuard {
    pub fn spawn(socket_path: &Path, spec: &WorkspaceSpec) -> Self {
        let manager = new_shared_manager();
        {
            let mut guard = manager.lock().expect("lease manager lock");
            for (key, limiter) in &spec.limiters {
                guard.set_capacity(
                    key.name.clone(),
                    key.scope.clone(),
                    key.scope_key.clone(),
                    limiter_capacity(limiter),
                );
            }
            for (key, queue) in &spec.queues {
                guard.set_capacity(
                    key.name.clone(),
                    key.scope.clone(),
                    key.scope_key.clone(),
                    queue_capacity(queue),
                );
            }
        }

        let runtime = Runtime::new().expect("tokio runtime");
        let manager = Arc::clone(&manager);
        let socket_path = socket_path.to_path_buf();
        let serve_path = socket_path.clone();
        let task = runtime.spawn(async move {
            let _ = run_server(&serve_path, manager).await;
        });

        let deadline = Instant::now() + Duration::from_secs(5);
        while !socket_path.exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for local daemon socket {}",
                socket_path.display()
            );
            thread::sleep(Duration::from_millis(20));
        }

        Self {
            runtime,
            task,
            socket_path,
        }
    }
}

impl Drop for LocalDaemonGuard {
    fn drop(&mut self) {
        self.task.abort();
        self.runtime.block_on(async {
            tokio::time::sleep(Duration::from_millis(20)).await;
        });
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

fn limiter_capacity(limiter: &LimiterDef) -> f64 {
    match limiter {
        LimiterDef::Resource { capacity, .. } => *capacity,
        LimiterDef::Lock { .. } => 1.0,
        LimiterDef::RateLimit { burst, .. } => *burst as f64,
        LimiterDef::ProcessCap { max_running, .. } => *max_running as f64,
    }
}

fn queue_capacity(queue: &QueueDef) -> f64 {
    queue.slots as f64
}
