use tak_core::v2::RunSubmission;
use takd::RunStore;

use super::super::{v2_cluster, v2_run, worker_http};

pub struct V2MixedOutputCluster {
    pub(super) worker: worker_http::RunningServer,
    _origin: v2_cluster::Origin,
    pub store: RunStore,
    _guard: v2_cluster::ClusterGuard,
}

impl V2MixedOutputCluster {
    pub async fn start() -> Self {
        let guard = v2_cluster::cluster_lock().await;
        let worker = worker_http::start_server().await;
        let workers = [v2_cluster::WorkerSpec::direct("builder-a", worker.addr, 2)];
        let peers = v2_cluster::peers(&workers);
        let origin = v2_cluster::Origin::start(peers, takd::TorBroker::new()).await;
        let store = origin.store.clone();
        Self {
            worker,
            _origin: origin,
            store,
            _guard: guard,
        }
    }

    pub async fn run(&self, request: &RunSubmission) -> String {
        let run_id = v2_run::scheduler::commit(&self.store, request, "alice");
        self._origin.wait_for_terminal(&run_id).await;
        run_id
    }
}
