use std::collections::BTreeMap;

use std::path::PathBuf;

use tak_core::model::{
    ContainerMountSpec, ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec,
};

use crate::container_engine::ContainerEngine;
use crate::{ContainerExecutionIdentity, ImageCacheOptions};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionMetadata {
    pub(crate) kind: String,
    pub(crate) engine: Option<String>,
    pub(crate) node_id: String,
    pub(crate) env_overrides: BTreeMap<String, String>,
    pub(crate) container_plan: Option<ContainerExecutionPlan>,
    pub(crate) container_identity: Option<ContainerExecutionIdentity>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerExecutionPlan {
    pub(crate) engine: ContainerEngine,
    pub(crate) source: ContainerRuntimeSourceSpec,
    pub(crate) image: String,
    pub(crate) container_user: Option<String>,
    pub(crate) image_cache: Option<ImageCachePlan>,
    pub(crate) mounts: Vec<ContainerMountSpec>,
    pub(crate) private_root: Option<PathBuf>,
    pub(crate) resource_limits: Option<ContainerResourceLimitsSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageCachePlan {
    pub(crate) options: ImageCacheOptions,
    pub(crate) cache_key: String,
    pub(crate) source_kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerLifecycleStage {
    Pull,
    Start,
    Runtime,
}

impl ContainerLifecycleStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Start => "start",
            Self::Runtime => "runtime",
        }
    }
}
