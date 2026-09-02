use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};

use anyhow::Result;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

type TransferKey = (String, String);
type TransferLock = AsyncMutex<()>;

#[derive(Default)]
pub(super) struct WorkspaceTransfers {
    entries: Mutex<BTreeMap<TransferKey, Weak<TransferLock>>>,
}

impl WorkspaceTransfers {
    pub(super) async fn acquire(
        &self,
        node_id: &str,
        fingerprint: &str,
    ) -> Result<OwnedMutexGuard<()>> {
        let key = (node_id.to_owned(), fingerprint.to_owned());
        let transfer = {
            let mut entries = self
                .entries
                .lock()
                .map_err(|_| anyhow::anyhow!("workspace transfer registry lock poisoned"))?;
            entries.retain(|_, entry| entry.strong_count() > 0);
            match entries.get(&key).and_then(Weak::upgrade) {
                Some(transfer) => transfer,
                None => {
                    let transfer = Arc::new(AsyncMutex::new(()));
                    entries.insert(key, Arc::downgrade(&transfer));
                    transfer
                }
            }
        };
        Ok(transfer.lock_owned().await)
    }
}
