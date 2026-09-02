use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::local_daemon::v2::{RemoteInventoryEntry, RemoteStatusEntry};

use super::peer_manager::PeerManager;
use super::protocol::TorBroker;

mod inventory;
mod onboarding;
mod status;

#[derive(Clone)]
pub(crate) struct RemoteAccess {
    inventory: inventory::InventoryFile,
    broker: TorBroker,
    peers: PeerManager,
    mutation: Arc<tokio::sync::Mutex<()>>,
}

pub(crate) enum RemoteAccessError {
    UnsupportedInvite,
    ProtocolMismatch,
    Failed(anyhow::Error),
}

impl RemoteAccess {
    pub(crate) fn new(path: PathBuf, broker: TorBroker, peers: PeerManager) -> Result<Self> {
        let inventory_exists = path.exists();
        let inventory = inventory::InventoryFile::new(path);
        if inventory_exists {
            peers.apply_inventory(inventory.load()?);
        }
        Ok(Self {
            inventory,
            broker,
            peers,
            mutation: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    pub(crate) async fn preview(
        &self,
        invite: &str,
    ) -> Result<RemoteInventoryEntry, RemoteAccessError> {
        onboarding::resolve(self, invite).await.map(public_remote)
    }

    pub(crate) async fn add(
        &self,
        invite: &str,
    ) -> Result<RemoteInventoryEntry, RemoteAccessError> {
        let remote = onboarding::resolve(self, invite).await?;
        {
            let _guard = self.mutation.lock().await;
            let mut inventory = self.inventory.load().map_err(RemoteAccessError::Failed)?;
            inventory
                .remotes
                .retain(|configured| configured.node_id != remote.node_id);
            inventory.remotes.push(remote.clone());
            inventory
                .remotes
                .sort_by(|left, right| left.node_id.cmp(&right.node_id));
            self.persist_and_apply(&inventory)
                .map_err(RemoteAccessError::Failed)?;
        }
        let snapshot_recorded = self
            .peers
            .probe_worker_once(&self.broker, &remote.node_id)
            .await;
        if !snapshot_recorded {
            self.peers
                .probe_worker_once(&self.broker, &remote.node_id)
                .await;
        }
        Ok(public_remote(remote))
    }

    pub(crate) fn list(&self) -> Result<Vec<RemoteInventoryEntry>> {
        let mut remotes = self
            .inventory
            .load()?
            .remotes
            .into_iter()
            .map(public_remote)
            .collect::<Vec<_>>();
        remotes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        Ok(remotes)
    }

    pub(crate) async fn remove(&self, node_id: &str) -> Result<bool> {
        let _guard = self.mutation.lock().await;
        let mut inventory = self.inventory.load()?;
        let before = inventory.remotes.len();
        inventory.remotes.retain(|remote| remote.node_id != node_id);
        let removed = inventory.remotes.len() != before;
        if removed {
            self.persist_and_apply(&inventory)?;
        }
        Ok(removed)
    }

    pub(crate) async fn statuses(&self, node_ids: &[String]) -> Result<Vec<RemoteStatusEntry>> {
        status::snapshot(self, node_ids).await
    }

    pub(crate) async fn read(&self, node_id: &str, path: &str) -> Result<(u16, Vec<u8>)> {
        status::read(self, node_id, path).await
    }

    fn configured(&self) -> Result<RemoteInventory> {
        self.inventory.load()
    }

    fn persist_and_apply(&self, inventory: &RemoteInventory) -> Result<()> {
        self.inventory.save(inventory)?;
        self.peers.apply_inventory(inventory.clone());
        Ok(())
    }
}

fn public_remote(remote: RemoteRecord) -> RemoteInventoryEntry {
    RemoteInventoryEntry {
        node_id: remote.node_id,
        display_name: remote.display_name,
        base_url: remote.base_url,
        pools: remote.pools,
        tags: remote.tags,
        capabilities: remote.capabilities,
        transport: remote.transport,
        enabled: remote.enabled,
    }
}

impl From<anyhow::Error> for RemoteAccessError {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}
