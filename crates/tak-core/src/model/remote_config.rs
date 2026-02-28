#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<RemoteTransportDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<RemoteWorkspaceDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<RemoteResultDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RemoteRuntimeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTransportDef {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<ServiceAuthDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAuthDef {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTransportKind {
    DirectHttps,
    Tor,
}

impl RemoteTransportKind {
    #[must_use]
    pub fn as_result_value(self) -> &'static str {
        match self {
            Self::DirectHttps => "direct",
            Self::Tor => "tor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWorkspaceDef {
    pub transfer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResultDef {
    pub sync: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRuntimeDef {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub mounts: Vec<ContainerMountDef>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "resources")]
    pub resource_limits: Option<ContainerResourceLimitsDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMountDef {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceLimitsDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u64>,
}

