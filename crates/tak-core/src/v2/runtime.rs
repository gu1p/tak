use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod mount;

pub use mount::ContainerMount;
use mount::validate_mounts;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContainerSource {
    Image {
        image: String,
    },
    Dockerfile {
        dockerfile: String,
        build_context: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResources {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskRuntime {
    Container {
        source: ContainerSource,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        mounts: Vec<ContainerMount>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resources: Option<RuntimeResources>,
    },
}

impl TaskRuntime {
    #[must_use]
    pub fn container(source: ContainerSource) -> Self {
        Self::Container {
            source,
            mounts: Vec::new(),
            env: BTreeMap::new(),
            resources: None,
        }
    }

    pub fn configured_container(
        source: ContainerSource,
        mut mounts: Vec<ContainerMount>,
        env: BTreeMap<String, String>,
        resources: Option<RuntimeResources>,
    ) -> Result<Self, String> {
        mounts.sort();
        mounts.dedup();
        let env = crate::model::normalize_runtime_env(&env).map_err(|error| error.to_string())?;
        let runtime = Self::Container {
            source,
            mounts,
            env,
            resources,
        };
        runtime.validate()?;
        Ok(runtime)
    }

    #[must_use]
    pub const fn resources(&self) -> Option<RuntimeResources> {
        match self {
            Self::Container { resources, .. } => *resources,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let Self::Container {
            source,
            mounts,
            env,
            resources,
        } = self;
        validate_source(source)?;
        validate_mounts(mounts)?;
        let canonical_env =
            crate::model::normalize_runtime_env(env).map_err(|error| error.to_string())?;
        if canonical_env != *env {
            return Err("container runtime environment is not canonical".into());
        }
        if resources.is_some_and(|value| value.cpu_millis == 0 || value.memory_bytes == 0) {
            return Err("container runtime resources must be positive".into());
        }
        Ok(())
    }
}

fn validate_source(source: &ContainerSource) -> Result<(), String> {
    match source {
        ContainerSource::Image { image } => {
            let canonical = crate::model::normalize_container_image_reference(image)
                .map_err(|error| format!("invalid container image: {error}"))?;
            if canonical.canonical != *image {
                return Err("container image is not canonical".into());
            }
        }
        ContainerSource::Dockerfile {
            dockerfile,
            build_context,
        } => {
            validate_path(dockerfile, "dockerfile")?;
            validate_path(build_context, "build context")?;
            if !within(dockerfile, build_context) {
                return Err("container dockerfile must be within build context".into());
            }
        }
    }
    Ok(())
}

fn validate_path(value: &str, field: &str) -> Result<(), String> {
    let normalized = crate::model::normalize_path_ref("workspace", value)
        .map_err(|error| format!("invalid container {field}: {error}"))?;
    if normalized.path != value {
        return Err(format!("container {field} is not canonical"));
    }
    Ok(())
}

fn within(path: &str, root: &str) -> bool {
    root == "."
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}
