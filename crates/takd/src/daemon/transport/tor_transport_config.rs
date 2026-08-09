use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtiSettings {
    pub socks5_addr: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorTransportConfig {
    pub onion_endpoint: String,
    pub service_auth_token: String,
    pub arti: ArtiSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorHiddenServiceRuntimeConfig {
    pub nickname: String,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}
