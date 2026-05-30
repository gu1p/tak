use std::path::PathBuf;

use arti_client::TorClient;
use tor_rtcompat::PreferredRuntime;

use super::TorBroker;

impl TorBroker {
    // Build a broker that borrows the hidden service's Tor client (published
    // later via `set_shared_tor_client`) instead of bootstrapping its own. Used
    // on the `tor` transport so a single Arti client serves the onion AND dials
    // peers, rather than two clients double-bootstrapping and contending for one
    // state-directory lock (which left every peer permanently unreachable).
    pub(crate) fn for_shared_tor_client(state_root: PathBuf) -> Self {
        Self::with_options(super::test_tor_onion_dial_addr(), Some(state_root), true)
    }

    // Publish the hidden service's Tor client so peer dials reuse it. Called on
    // each onion session (including restarts) so the broker always borrows the
    // current client.
    pub(crate) fn set_shared_tor_client(&self, client: TorClient<PreferredRuntime>) {
        *self
            .inner
            .shared_tor_client
            .lock()
            .expect("shared tor client lock poisoned") = Some(client);
    }

    pub(super) fn shared_tor_client_snapshot(&self) -> Option<TorClient<PreferredRuntime>> {
        self.inner
            .shared_tor_client
            .lock()
            .expect("shared tor client lock poisoned")
            .clone()
    }

    pub(super) fn requires_shared_client(&self) -> bool {
        self.inner.requires_shared_client
    }
}
