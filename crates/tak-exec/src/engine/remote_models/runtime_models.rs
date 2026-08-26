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
    /// Declared CPU/memory reservations for the task. Threaded to the container
    /// runtime to enforce CPU as a real cgroup quota (`nano_cpus`) and to cap
    /// test/codegen parallelism. Memory is NEVER applied as a hard cgroup cap:
    /// that would let the kernel OOM-kill the container for over-using memory,
    /// which Tak must not do. Memory pressure is handled by throttling and
    /// admission, not by killing containers.
    pub(crate) resource_limits: Option<ContainerResourceLimitsSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageCachePlan {
    pub(crate) options: ImageCacheOptions,
    pub(crate) cache_key: String,
    pub(crate) source_kind: String,
}
