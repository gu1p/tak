use std::path::Path;

use anyhow::{Result, anyhow, bail};
use tak_core::model::{normalize_container_image_reference, normalize_path_ref};
use tak_core::v2::{
    ContainerSource, Execution, LocalExecution, RemoteExecution, RemoteSelection, Session,
    TaskRuntime,
};

use super::RunCliArgs;

#[derive(Clone, Copy)]
enum Placement {
    Local,
    LocalNoContainer,
    Remote,
}

pub(super) struct ExecutionOverride {
    placement: Placement,
    explicit_runtime: Option<TaskRuntime>,
    require_container: bool,
}

pub(super) fn resolve(args: &RunCliArgs) -> Result<Option<ExecutionOverride>> {
    let placement = placement(args)?;
    validate_container_flags(args, placement)?;
    let Some(placement) = placement else {
        return Ok(None);
    };
    Ok(Some(ExecutionOverride {
        placement,
        explicit_runtime: source(args)?.map(TaskRuntime::container),
        require_container: args.container || matches!(placement, Placement::Remote),
    }))
}

impl ExecutionOverride {
    pub(super) fn execution(&self, authored: Option<&Execution>) -> Result<Execution> {
        let runtime = match self.placement {
            Placement::LocalNoContainer => None,
            _ => self
                .explicit_runtime
                .clone()
                .or_else(|| authored.and_then(Execution::runtime).cloned()),
        };
        if self.require_container && runtime.is_none() {
            bail!(
                "execution override requires a container runtime; declare one or pass a container source flag"
            )
        }
        Ok(match self.placement {
            Placement::Local | Placement::LocalNoContainer => Execution::LocalOnly {
                local: LocalExecution {
                    reason: "command-line local override".into(),
                    session: None,
                    runtime,
                },
            },
            Placement::Remote => Execution::RemoteOnly {
                remote: remote_template(authored, runtime),
            },
        })
    }

    pub(super) fn session(
        &self,
        session: Option<Session>,
        execution: &Execution,
    ) -> Option<Session> {
        session.map(|mut session| {
            session.execution = Some(Box::new(execution.clone()));
            session
        })
    }
}

fn placement(args: &RunCliArgs) -> Result<Option<Placement>> {
    match (args.local, args.local_no_container, args.remote) {
        (_, true, true) => bail!("--local-no-container and --remote are mutually exclusive"),
        (true, false, true) => bail!("--local and --remote are mutually exclusive"),
        (_, true, false) => Ok(Some(Placement::LocalNoContainer)),
        (true, false, false) => Ok(Some(Placement::Local)),
        (false, false, true) => Ok(Some(Placement::Remote)),
        (false, false, false) => Ok(None),
    }
}

fn validate_container_flags(args: &RunCliArgs, placement: Option<Placement>) -> Result<()> {
    if args.container_image.is_some() && args.container_dockerfile.is_some() {
        bail!("--container-image and --container-dockerfile are mutually exclusive")
    }
    if args.container_build_context.is_some() && args.container_dockerfile.is_none() {
        bail!("--container-build-context requires --container-dockerfile")
    }
    let source = args.container_image.is_some() || args.container_dockerfile.is_some();
    if matches!(placement, Some(Placement::LocalNoContainer)) && (args.container || source) {
        bail!("--local-no-container cannot be combined with container flags")
    }
    if source && !args.container && !matches!(placement, Some(Placement::Remote)) {
        bail!("container source flags require --remote or --container")
    }
    if args.container && placement.is_none() {
        bail!("--container requires exactly one of --local or --remote")
    }
    Ok(())
}

fn source(args: &RunCliArgs) -> Result<Option<ContainerSource>> {
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
    Ok(Some(ContainerSource::Dockerfile {
        dockerfile,
        build_context: normalized_path(&context, "--container-build-context")?,
    }))
}

fn normalized_path(value: &str, flag: &str) -> Result<String> {
    Ok(normalize_path_ref("workspace", value)
        .map_err(|error| anyhow!("invalid {flag}: {error}"))?
        .path)
}

fn remote_template(authored: Option<&Execution>, runtime: Option<TaskRuntime>) -> RemoteExecution {
    let mut remote = authored
        .and_then(find_remote)
        .cloned()
        .unwrap_or(RemoteExecution {
            reason: "command-line remote override".into(),
            pool: None,
            required_tags: Vec::new(),
            required_capabilities: Vec::new(),
            transport: None,
            selection: RemoteSelection::Balanced,
            session: None,
            runtime: None,
        });
    remote.session = None;
    remote.runtime = runtime;
    remote
}

fn find_remote(execution: &Execution) -> Option<&RemoteExecution> {
    match execution {
        Execution::RemoteOnly { remote } => Some(remote),
        Execution::FirstAvailable { placements, .. } => placements.iter().find_map(find_remote),
        Execution::LocalOnly { .. } => None,
    }
}
