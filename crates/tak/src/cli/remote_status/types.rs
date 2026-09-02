use super::RemoteRecord;

#[derive(Clone, Debug)]
pub(in crate::cli) struct RemoteStatusResult {
    pub(in crate::cli) remote: RemoteRecord,
    pub(in crate::cli) status: Option<tak_proto::NodeStatusResponse>,
    pub(in crate::cli) error: Option<String>,
    pub(in crate::cli) peer: Option<DaemonPeerSnapshot>,
}

pub(in crate::cli) type DaemonPeerSnapshot = tak_proto::local_daemon::v2::RemotePeerHealth;
