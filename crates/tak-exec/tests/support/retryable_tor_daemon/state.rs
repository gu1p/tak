#[derive(Default)]
pub(in super::super) struct State {
    pub(super) non_retryable_peers: bool,
    pub(super) peer_requests: u32,
    pub(super) committed: u64,
    pub(super) size: u64,
    pub(super) drops_at_committed_offset: u8,
    pub(super) upload_ids: Vec<String>,
    pub(super) stream_offsets: Vec<u64>,
    pub(super) submit_attempts: Vec<u32>,
    pub(super) upload_failover: bool,
    pub(super) failover_results: bool,
    pub(super) submit_failover: bool,
    pub(super) submit_transport_failover: bool,
    pub(super) submit_always_fails: bool,
    pub(super) selected_node: String,
    pub(super) placement_exclusions: Vec<Vec<String>>,
}
