#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
    #[serde(default)]
    pub required_tags: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<RemoteTransportDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RemoteRuntimeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTransportDef {
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTransportKind {
    Any,
    Direct,
    Tor,
}

impl RemoteTransportKind {
    #[must_use]
    pub fn as_result_value(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Direct => "direct",
            Self::Tor => "tor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRuntimeDef {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<PathInputDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_context: Option<PathInputDef>,
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
