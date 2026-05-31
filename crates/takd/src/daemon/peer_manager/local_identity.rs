use tak_core::remote_inventory::RemoteInventory;

use super::PeerManager;

/// Identity of the node hosting a broker, used to keep the local node out of its
/// own peer set so a task submit is never relayed back to itself.
#[derive(Debug, Clone)]
pub struct LocalNodeIdentity {
    node_id: String,
    endpoint: Option<String>,
}

impl LocalNodeIdentity {
    /// Builds a local identity from a node id and its optional advertised endpoint.
    ///
    /// ```rust
    /// use takd::LocalNodeIdentity;
    ///
    /// let identity = LocalNodeIdentity::new("node-a".to_string(), None);
    /// assert!(identity.matches_peer("node-a", "http://node-a.onion"));
    /// assert!(!identity.matches_peer("node-b", "http://node-b.onion"));
    /// ```
    pub fn new(node_id: String, endpoint: Option<String>) -> Self {
        Self { node_id, endpoint }
    }

    /// Reports whether a peer record refers to this same local node, matching by
    /// node id or by an identical advertised endpoint.
    ///
    /// ```rust
    /// use takd::LocalNodeIdentity;
    ///
    /// let identity =
    ///     LocalNodeIdentity::new("self".to_string(), Some("http://self.onion".to_string()));
    /// assert!(identity.matches_peer("self", "http://elsewhere.onion"));
    /// assert!(identity.matches_peer("other", "http://self.onion"));
    /// assert!(!identity.matches_peer("other", "http://other.onion"));
    /// ```
    pub fn matches_peer(&self, node_id: &str, endpoint: &str) -> bool {
        self.node_id == node_id
            || self
                .endpoint
                .as_deref()
                .is_some_and(|local| local == endpoint)
    }
}

impl PeerManager {
    /// Builds a peer manager that excludes the local node from its own peer set.
    ///
    /// ```rust
    /// use takd::{LocalNodeIdentity, PeerManager};
    /// use tak_core::remote_inventory::RemoteInventory;
    ///
    /// let peers = PeerManager::from_inventory_with_local_identity(
    ///     RemoteInventory {
    ///         version: 1,
    ///         remotes: vec![],
    ///     },
    ///     LocalNodeIdentity::new("node-self".to_string(), None),
    /// );
    /// assert!(peers.snapshots().is_empty());
    /// ```
    pub fn from_inventory_with_local_identity(
        inventory: RemoteInventory,
        identity: LocalNodeIdentity,
    ) -> Self {
        let manager = Self::default();
        manager.set_local_identity(identity);
        manager.apply_inventory(inventory);
        manager
    }

    pub(crate) fn set_local_identity(&self, identity: LocalNodeIdentity) {
        self.lock_state().local_identity = Some(identity);
    }
}
