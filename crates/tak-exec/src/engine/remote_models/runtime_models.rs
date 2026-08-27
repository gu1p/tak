use std::collections::BTreeMap;

use tak_core::model::{ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec};

use crate::ImageCacheOptions;
use crate::container_engine::ContainerEngine;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionMetadata {
    pub(crate) kind: String,
    pub(crate) engine: Option<String>,
    pub(crate) env_overrides: BTreeMap<String, String>,
    pub(crate) container_plan: Option<ContainerExecutionPlan>,
    pub(crate) container_identity: Option<super::super::ContainerExecutionIdentity>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerExecutionPlan {
    pub(crate) engine: ContainerEngine,
    pub(crate) source: ContainerRuntimeSourceSpec,
    pub(crate) image: String,
    pub(crate) container_user: Option<String>,
    pub(crate) image_cache: Option<ImageCachePlan>,
    /// Authored CPU/memory reservations used for aggregate worker admission.
    /// They are estimates, not per-container CPU or memory caps; worker-wide
    /// cgroup fencing and pressure recovery protect the host as a whole.
    pub(crate) resource_limits: Option<ContainerResourceLimitsSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageCachePlan {
    pub(crate) options: ImageCacheOptions,
    pub(crate) cache_key: String,
    pub(crate) source_kind: String,
}
