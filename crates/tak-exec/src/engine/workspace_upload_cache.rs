//! Per-job (per `tak run`) cache of workspace uploads, so a byte-identical workspace is
//! uploaded to a given remote node only once and reused by every later task in the job.
//!
//! Keyed by `(node_id, workspace_content_hash)` (see [`workspace_content_hash`]). The first
//! task to need a given workspace on a node uploads it (the "leader"); concurrent tasks with
//! the same key wait on the leader (single-flight) and reuse the resulting
//! [`tak_proto::WorkspaceUploadRef`]; later tasks find a completed entry and skip both staging
//! and upload entirely.
//!
//! The map lives behind a `std::sync::Mutex` and is only ever locked for short, synchronous
//! sections — the single-flight *wait* happens on a `tokio::sync::watch` channel with no lock
//! held, and [`UploadLeadGuard`]'s `Drop` can therefore clean up a failed slot synchronously.
//!
//! [`workspace_content_hash`]: super::workspace_collect::workspace_content_hash

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tak_proto::WorkspaceUploadRef;
use tokio::sync::watch;

/// `(node_id, workspace_content_hash)`.
type Key = (String, String);

/// A completed upload that later tasks can reference instead of re-uploading.
#[derive(Clone, Debug)]
pub(crate) struct CachedUpload {
    pub(crate) upload: WorkspaceUploadRef,
    /// The worker the daemon actually stored the blob on (for daemon-tor placement this may
    /// differ from the requested node id); must be replayed as `x-tak-preferred-node` so a
    /// reused submit routes to the worker that holds the blob.
    pub(crate) preferred_node_id: Option<String>,
    /// Size of the uploaded archive, for upload progress reporting on reuse.
    pub(crate) archive_byte_len: u64,
}

enum Slot {
    /// An upload is in progress; receivers observe `Some` once it completes, or a closed
    /// channel if the leader failed (then waiters re-claim).
    InFlight(watch::Receiver<Option<CachedUpload>>),
    Done(CachedUpload),
}

#[derive(Clone, Default)]
pub(crate) struct SharedWorkspaceUploadCache {
    inner: Arc<Mutex<HashMap<Key, Slot>>>,
}

impl std::fmt::Debug for SharedWorkspaceUploadCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.lock().map(|map| map.len()).unwrap_or(0);
        f.debug_struct("SharedWorkspaceUploadCache")
            .field("entries", &len)
            .finish()
    }
}

/// Outcome of [`SharedWorkspaceUploadCache::claim`].
pub(crate) enum UploadClaim {
    /// A completed upload already exists (from cache or a sibling) — reference it.
    Reuse(CachedUpload),
    /// This caller is the leader and must perform the upload, then call
    /// [`UploadLeadGuard::publish`] on success (or drop the guard on failure).
    Lead(UploadLeadGuard),
}

/// Held by the single-flight leader while it uploads. Dropping it without [`publish`] (e.g.
/// on upload failure) clears the in-flight slot so waiters re-claim and retry.
///
/// [`publish`]: UploadLeadGuard::publish
pub(crate) struct UploadLeadGuard {
    cache: SharedWorkspaceUploadCache,
    key: Key,
    tx: watch::Sender<Option<CachedUpload>>,
    published: bool,
}

impl SharedWorkspaceUploadCache {
    /// Returns a completed upload for `key` if one exists. Does not wait on an in-flight
    /// upload — used to decide whether staging can be skipped up front.
    ///
    /// ```no_run
    /// # // Reason: This cache helper is exercised through remote workspace-upload tests.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn peek(&self, key: &Key) -> Option<CachedUpload> {
        let map = self.inner.lock().expect("upload cache mutex poisoned");
        match map.get(key) {
            Some(Slot::Done(cached)) => Some(cached.clone()),
            _ => None,
        }
    }

    /// Drops a completed entry (e.g. after a referenced blob turned out to be missing on the
    /// node), so the next claim re-uploads. Leaves an in-flight entry alone.
    ///
    /// ```no_run
    /// # // Reason: This cache helper is exercised through remote workspace-upload tests.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn invalidate(&self, key: &Key) {
        let mut map = self.inner.lock().expect("upload cache mutex poisoned");
        if matches!(map.get(key), Some(Slot::Done(_))) {
            map.remove(key);
        }
    }

    /// Claims responsibility for `key`: reuse an existing/just-finished upload, or become the
    /// single-flight leader. Followers wait on the leader with no lock held; if the leader
    /// fails (guard dropped without publishing) they re-claim and one becomes the new leader.
    ///
    /// ```no_run
    /// # // Reason: This async cache helper coordinates test uploads and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) async fn claim(&self, key: Key) -> UploadClaim {
        loop {
            let mut receiver = {
                let mut map = self.inner.lock().expect("upload cache mutex poisoned");
                match map.get(&key) {
                    Some(Slot::Done(cached)) => return UploadClaim::Reuse(cached.clone()),
                    Some(Slot::InFlight(rx)) => rx.clone(),
                    None => {
                        let (tx, rx) = watch::channel(None);
                        map.insert(key.clone(), Slot::InFlight(rx));
                        return UploadClaim::Lead(UploadLeadGuard {
                            cache: self.clone(),
                            key,
                            tx,
                            published: false,
                        });
                    }
                }
            };
            // Follower: wait for the leader without holding the map lock.
            loop {
                if let Some(cached) = receiver.borrow().clone() {
                    return UploadClaim::Reuse(cached);
                }
                if receiver.changed().await.is_err() {
                    break; // leader dropped without publishing → re-claim
                }
            }
        }
    }
}

impl UploadLeadGuard {
    /// Records the completed upload and wakes any waiters so they reuse it.
    ///
    /// ```no_run
    /// # // Reason: This guard is constructed by the upload cache claim path.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn publish(mut self, cached: CachedUpload) {
        {
            let mut map = self
                .cache
                .inner
                .lock()
                .expect("upload cache mutex poisoned");
            map.insert(self.key.clone(), Slot::Done(cached.clone()));
        }
        let _ = self.tx.send(Some(cached));
        self.published = true;
    }
}

impl Drop for UploadLeadGuard {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        // Leader failed (or produced an uncacheable inline result): clear the slot so waiters
        // re-claim. Dropping `tx` afterwards closes the channel, waking current waiters.
        let mut map = self
            .cache
            .inner
            .lock()
            .expect("upload cache mutex poisoned");
        if matches!(map.get(&self.key), Some(Slot::InFlight(_))) {
            map.remove(&self.key);
        }
    }
}
