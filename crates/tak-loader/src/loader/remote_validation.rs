use anyhow::{Result, anyhow, bail};
use tak_core::model::{
    ContainerRuntimeSourceInputSpec, ContainerRuntimeSourceSpec, PathInputDef, PathRef,
    RemoteRuntimeDef, RemoteRuntimeSpec, RemoteTransportDef, RemoteTransportKind,
    normalize_path_ref, validate_container_runtime_execution_spec,
};

use super::{
    V1_TRANSPORT_ANY, V1_TRANSPORT_DIRECT, V1_TRANSPORT_TOR,
    context_resolution::resolve_context_path,
};

pub(crate) fn validate_remote_transport(
    transport: Option<RemoteTransportDef>,
) -> Result<RemoteTransportKind> {
    let Some(transport) = transport else {
        return Ok(RemoteTransportKind::Any);
    };
    let kind = transport.kind.trim();
    if kind.is_empty() {
        bail!("execution Remote.transport.kind cannot be empty");
    }

    match kind {
        V1_TRANSPORT_ANY => Ok(RemoteTransportKind::Any),
        V1_TRANSPORT_DIRECT => Ok(RemoteTransportKind::Direct),
        V1_TRANSPORT_TOR => Ok(RemoteTransportKind::Tor),
        _ => bail!(
            "execution Remote.transport.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_ANY}`, `{V1_TRANSPORT_DIRECT}`, or `{V1_TRANSPORT_TOR}`"
        ),
    }
}

pub(crate) fn validate_runtime(
    runtime: Option<RemoteRuntimeDef>,
    package: &str,
    owner: &str,
) -> Result<Option<RemoteRuntimeSpec>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };

    if runtime.kind.trim() == "host" {
        bail!("Runtime.Host() is only valid for Execution.Local");
    }

    let validated = validate_container_runtime_execution_spec(&runtime)
        .map_err(|err| anyhow!("execution {owner}.runtime {err}"))?;
    let source = resolve_container_runtime_source(validated.source, package, owner)?;

    Ok(Some(RemoteRuntimeSpec::Containerized { source }))
}

pub(crate) fn validate_local_runtime(
    runtime: Option<RemoteRuntimeDef>,
    package: &str,
    owner: &str,
) -> Result<Option<RemoteRuntimeSpec>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };
    if runtime.kind.trim() == "host" {
        return Ok(None);
    }
    validate_runtime(Some(runtime), package, owner)
}

fn resolve_container_runtime_source(
    source: ContainerRuntimeSourceInputSpec,
    package: &str,
    owner: &str,
) -> Result<ContainerRuntimeSourceSpec> {
    match source {
        ContainerRuntimeSourceInputSpec::Image { image } => {
            Ok(ContainerRuntimeSourceSpec::Image { image })
        }
        ContainerRuntimeSourceInputSpec::Dockerfile {
            dockerfile,
            build_context,
        } => {
            let dockerfile = resolve_runtime_path(dockerfile, package, owner, "dockerfile")?;
            let build_context = match build_context {
                Some(build_context) => {
                    resolve_runtime_path(build_context, package, owner, "build_context")?
                }
                None => package_root_path(package)?,
            };

            if !is_path_within(&dockerfile, &build_context) {
                bail!("execution {owner}.runtime.dockerfile must be within build_context");
            }

            Ok(ContainerRuntimeSourceSpec::Dockerfile {
                dockerfile,
                build_context,
            })
        }
    }
}

fn resolve_runtime_path(
    path: PathInputDef,
    package: &str,
    owner: &str,
    field: &str,
) -> Result<PathRef> {
    resolve_context_path(path, package)
        .map_err(|err| anyhow!("execution {owner}.runtime.{field} {err}"))
}

fn package_root_path(package: &str) -> Result<PathRef> {
    let package_relative = package.trim_start_matches("//");
    let raw = if package_relative.is_empty() {
        "."
    } else {
        package_relative
    };
    normalize_path_ref("workspace", raw)
        .map_err(|err| anyhow!("invalid default package build context: {err}"))
}

fn is_path_within(path: &PathRef, root: &PathRef) -> bool {
    if path.anchor != root.anchor {
        return false;
    }
    if root.path == "." {
        return true;
    }
    if path.path == root.path {
        return true;
    }
    path.path
        .strip_prefix(&root.path)
        .is_some_and(|suffix| suffix.starts_with('/'))
}
