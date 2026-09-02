use std::path::Path;

use tak_core::model::{normalize_container_image_reference, normalize_path_ref};
use tak_core::v2::{ContainerSource, TaskRuntime};

use super::{ExecCliArgs, *};

pub(super) fn selected(args: &ExecCliArgs) -> Result<Option<TaskRuntime>> {
    validate_flags(args)?;
    let runtime = source(args)?.map(TaskRuntime::container);
    if (args.remote || args.container) && runtime.is_none() {
        bail!(
            "tak exec requires --container-image or --container-dockerfile for container execution"
        );
    }
    Ok(runtime)
}

fn validate_flags(args: &ExecCliArgs) -> Result<()> {
    if args.local_no_container && args.remote {
        bail!("--local-no-container and --remote are mutually exclusive");
    }
    if args.local_no_container
        && (args.container
            || args.container_image.is_some()
            || args.container_dockerfile.is_some()
            || args.container_build_context.is_some())
    {
        bail!("--local-no-container cannot be combined with container flags");
    }
    if args.container_image.is_some() && args.container_dockerfile.is_some() {
        bail!("--container-image and --container-dockerfile are mutually exclusive");
    }
    if args.container_build_context.is_some() && args.container_dockerfile.is_none() {
        bail!("--container-build-context requires --container-dockerfile");
    }
    let has_source = args.container_image.is_some() || args.container_dockerfile.is_some();
    if has_source && !args.remote && !args.container {
        bail!("container source flags require --remote or --container");
    }
    if args.container && !args.local && !args.remote {
        bail!("--container requires exactly one of --local or --remote");
    }
    Ok(())
}

fn source(args: &ExecCliArgs) -> Result<Option<ContainerSource>> {
    if let Some(image) = &args.container_image {
        let image = normalize_container_image_reference(image)
            .map_err(|error| anyhow!("invalid --container-image: {error}"))?
            .canonical;
        return Ok(Some(ContainerSource::Image { image }));
    }
    let Some(dockerfile) = &args.container_dockerfile else {
        return Ok(None);
    };
    let dockerfile = normalized_path(dockerfile, "--container-dockerfile")?;
    let context = args.container_build_context.clone().unwrap_or_else(|| {
        Path::new(&dockerfile)
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .map_or_else(|| ".".into(), |path| path.to_string_lossy().into_owned())
    });
    let build_context = normalized_path(&context, "--container-build-context")?;
    Ok(Some(ContainerSource::Dockerfile {
        dockerfile,
        build_context,
    }))
}

fn normalized_path(value: &str, flag: &str) -> Result<String> {
    Ok(normalize_path_ref("workspace", value)
        .map_err(|error| anyhow!("invalid {flag}: {error}"))?
        .path)
}
