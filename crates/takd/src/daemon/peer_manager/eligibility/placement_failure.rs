#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementFailure {
    NoConfiguredPeers,
    NoMatchingPeers,
    AllPeersAuthFailed,
    AllPeersProtocolMismatch,
    AllPeersUnreachable,
    ResourceRequirementsExceedWorkerCapacity { diagnostic: String },
    NoPlaceablePeers,
}

impl PlacementFailure {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoConfiguredPeers => "no_configured_tor_peers",
            Self::NoMatchingPeers => "no_matching_tor_peers",
            Self::AllPeersAuthFailed => "all_tor_peers_auth_failed",
            Self::AllPeersProtocolMismatch => "all_tor_peers_protocol_mismatch",
            Self::AllPeersUnreachable => "all_tor_peers_unreachable",
            Self::ResourceRequirementsExceedWorkerCapacity { .. } => {
                "resource_requirements_exceed_worker_capacity"
            }
            Self::NoPlaceablePeers => "no_placeable_tor_peers",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::AllPeersUnreachable | Self::NoPlaceablePeers)
    }
}

impl std::fmt::Display for PlacementFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfiguredPeers => write!(f, "no configured Tor peers"),
            Self::NoMatchingPeers => {
                write!(
                    f,
                    "no Tor peers match pool/tag/capability/transport requirements"
                )
            }
            Self::AllPeersAuthFailed => write!(f, "all Tor peers are auth failed"),
            Self::AllPeersProtocolMismatch => write!(f, "all Tor peers have protocol mismatch"),
            Self::AllPeersUnreachable => write!(f, "all Tor peers are unreachable"),
            Self::ResourceRequirementsExceedWorkerCapacity { diagnostic } => {
                write!(f, "{diagnostic}")
            }
            Self::NoPlaceablePeers => write!(f, "no placeable Tor peers"),
        }
    }
}

impl std::error::Error for PlacementFailure {}
