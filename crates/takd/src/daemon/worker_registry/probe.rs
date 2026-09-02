use std::time::Duration;

use anyhow::{Result, bail};
use futures::stream::{FuturesUnordered, StreamExt};

use super::{WorkerConnectionTarget, WorkerProbeTarget, WorkerRegistry};
use crate::daemon::peer_manager::PeerManager;
use crate::daemon::protocol::TorBroker;

const PROBE_INTERVAL: Duration = Duration::from_secs(15);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

impl PeerManager {
    pub async fn probe_workers_once(&self, broker: &TorBroker) {
        self.workers.probe_once(broker).await;
    }

    pub(crate) async fn probe_worker_once(&self, broker: &TorBroker, node_id: &str) -> bool {
        self.workers.probe_node_once(broker, node_id).await
    }

    pub fn spawn_worker_v2_probe_loop(&self, broker: TorBroker) {
        let workers = self.workers.clone();
        tokio::spawn(async move {
            loop {
                workers.probe_once(&broker).await;
                tokio::time::sleep(PROBE_INTERVAL).await;
            }
        });
    }
}

impl WorkerRegistry {
    fn probe_target(&self, node_id: &str) -> Option<WorkerProbeTarget> {
        self.inner
            .lock()
            .expect("worker registry lock poisoned")
            .get(node_id)
            .map(|entry| WorkerProbeTarget {
                connection: super::selection::connection_target(entry),
                generation: entry.connection_generation,
            })
    }

    async fn probe_node_once(&self, broker: &TorBroker, node_id: &str) -> bool {
        let Some(target) = self.probe_target(node_id) else {
            return false;
        };
        let (target, result) = probe(broker.clone(), target).await;
        self.record_probe(target, result)
    }

    async fn probe_once(&self, broker: &TorBroker) {
        let mut probes = self
            .probe_targets()
            .into_iter()
            .map(|target| probe(broker.clone(), target))
            .collect::<FuturesUnordered<_>>();
        while let Some((target, result)) = probes.next().await {
            self.record_probe(target, result);
        }
    }

    fn record_probe(
        &self,
        target: WorkerProbeTarget,
        result: Result<tak_proto::worker_v2::WorkerSnapshot>,
    ) -> bool {
        match result {
            Ok(snapshot) => self.mark_probe_snapshot(&target, snapshot),
            Err(error) => {
                let node_id = &target.connection.node_id;
                tracing::debug!(node_id, error = %error, "worker v2 probe failed");
                self.mark_probe_failure(&target);
                false
            }
        }
    }
}

async fn probe(
    broker: TorBroker,
    target: WorkerProbeTarget,
) -> (
    WorkerProbeTarget,
    Result<tak_proto::worker_v2::WorkerSnapshot>,
) {
    let result = tokio::time::timeout(PROBE_TIMEOUT, probe_inner(&broker, &target.connection))
        .await
        .map_err(|_| anyhow::anyhow!("worker snapshot timed out"))
        .and_then(|result| result);
    (target, result)
}

async fn probe_inner(
    broker: &TorBroker,
    target: &WorkerConnectionTarget,
) -> Result<tak_proto::worker_v2::WorkerSnapshot> {
    validate_transport(target)?;
    let response = broker
        .worker_v2_http_exchange(target, "GET", "/v2/worker/snapshot", &[])
        .await?;
    if response.status != 200 {
        bail!("worker snapshot returned HTTP {}", response.status);
    }
    let snapshot = tak_proto::worker_v2::decode_snapshot(&response.body)?;
    if snapshot.node_id != target.node_id {
        bail!("worker snapshot node id mismatch");
    }
    Ok(snapshot)
}

fn validate_transport(target: &WorkerConnectionTarget) -> Result<()> {
    let (host, _) = tak_core::endpoint::endpoint_host_port(&target.endpoint)?;
    match (target.transport.as_str(), host.ends_with(".onion")) {
        ("direct", false) | ("tor", true) => Ok(()),
        ("direct" | "tor", _) => bail!("worker transport does not match its endpoint"),
        _ => bail!("worker transport is unsupported"),
    }
}
