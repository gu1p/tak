use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use tak_core::model::{
    ContainerMountSpec, ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, RemoteRuntimeSpec,
    normalize_path_ref,
};
use tak_core::v2::{ContainerSource, TaskRuntime};

pub(super) fn insert_host_path_for_native_runtime(
    environment: &mut BTreeMap<String, String>,
    runtime: Option<&TaskRuntime>,
) {
    if runtime.is_none() {
        environment.insert(
            "PATH".into(),
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        );
    }
}

pub(super) fn runner_runtime(
    runtime: Option<&TaskRuntime>,
    private_root: &Path,
) -> Result<Option<RemoteRuntimeSpec>> {
    let Some(TaskRuntime::Container {
        source,
        mounts,
        env,
        resources,
    }) = runtime
    else {
        return Ok(None);
    };
    make_container_private_directories_writable(private_root)?;
    let source = match source {
        ContainerSource::Image { image } => ContainerRuntimeSourceSpec::Image {
            image: image.clone(),
        },
        ContainerSource::Dockerfile {
            dockerfile,
            build_context,
        } => ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile: normalize_path_ref("workspace", dockerfile)?,
            build_context: normalize_path_ref("workspace", build_context)?,
        },
    };
    let resource_limits = resources.map(|resources| ContainerResourceLimitsSpec {
        cpu_cores: Some(resources.cpu_millis as f64 / 1_000.0),
        memory_mb: Some(resources.memory_bytes / (1024 * 1024)),
    });
    Ok(Some(RemoteRuntimeSpec::ContainerizedV2 {
        source,
        mounts: mounts
            .iter()
            .map(|mount| ContainerMountSpec {
                source: mount.source.clone(),
                target: mount.target.clone(),
                read_only: mount.read_only,
            })
            .collect(),
        env: env.clone(),
        private_root: private_root.to_path_buf(),
        resource_limits,
    }))
}

fn make_container_private_directories_writable(private_root: &Path) -> Result<()> {
    for name in ["home", "tmp"] {
        let path = private_root.join(name);
        std::fs::create_dir_all(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777))?;
        }
    }
    Ok(())
}
