use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

mod container_engine_selection;
mod container_engine_types;
mod tor_transport_config;

pub use container_engine_selection::{select_container_engine, select_container_engine_with_probe};
pub use container_engine_types::{ContainerEngine, ContainerEngineProbe, HostPlatform};
pub use tor_transport_config::{
    ArtiSettings, TorHiddenServiceRuntimeConfig, TorTransportConfig,
    normalize_tor_transport_config, validate_tor_transport_config,
};
