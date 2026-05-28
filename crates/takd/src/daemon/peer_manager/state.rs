use super::PeerState;

impl PeerState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Degraded => "degraded",
            Self::AuthFailed => "auth_failed",
            Self::Unreachable => "unreachable",
            Self::ProtocolMismatch => "protocol_mismatch",
        }
    }
}
