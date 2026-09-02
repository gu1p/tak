use anyhow::{Context, Result};
use tak_core::model::{normalize_container_image_reference, normalize_path_ref};
use tak_core::v2::{ContainerSource, TaskRuntime};

use super::super::super::run_spec::DockerRunSpec;

pub(super) fn from_spec(spec: &DockerRunSpec) -> Result<TaskRuntime> {
    let source = match (&spec.image, &spec.dockerfile) {
        (Some(image), None) => ContainerSource::Image {
            image: normalize_container_image_reference(image)
                .with_context(|| format!("invalid container image `{image}`"))?
                .canonical,
        },
        (None, Some(dockerfile)) => ContainerSource::Dockerfile {
            dockerfile: workspace_path(dockerfile, "Dockerfile")?,
            build_context: workspace_path(
                spec.build_context.as_deref().unwrap_or("."),
                "build context",
            )?,
        },
        _ => unreachable!("docker run source was validated"),
    };
    Ok(TaskRuntime::container(source))
}

fn workspace_path(value: &str, field: &str) -> Result<String> {
    normalize_path_ref("workspace", value)
        .with_context(|| format!("invalid {field} path `{value}`"))
        .map(|path| path.path)
}
