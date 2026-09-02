use std::sync::OnceLock;

mod origin;
mod worker;

pub use origin::Origin;
pub use worker::{WorkerSpec, attempt_count, mark_snapshot, peers};

static CLUSTER_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

pub type ClusterGuard = tokio::sync::MutexGuard<'static, ()>;

pub async fn cluster_lock() -> ClusterGuard {
    CLUSTER_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}
