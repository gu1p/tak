use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerEngine {
    Docker,
    Podman,
}

impl ContainerEngine {
    #[must_use]
    pub(super) fn as_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    MacOs,
    Other,
}

impl HostPlatform {
    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

pub trait ContainerEngineProbe {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String>;
}
