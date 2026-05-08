use anyhow::{Result, bail};

use super::run_spec::DockerRunSpec;

pub(in crate::cli::docker_cli) fn validate_docker_run_spec(spec: &DockerRunSpec) -> Result<()> {
    if spec.image.is_none() && spec.dockerfile.is_none() {
        bail!("tak docker run requires an IMAGE or `-f Dockerfile`");
    }
    if spec.image.is_some() && spec.dockerfile.is_some() {
        bail!("tak docker run accepts either IMAGE or `-f Dockerfile`, not both");
    }
    if spec.argv.is_empty() {
        bail!(
            "tak docker run requires an explicit command; image default commands are not supported yet"
        );
    }
    if !spec.publishes.is_empty() {
        bail!(
            "tak docker run does not support port publishing yet; remote-to-local forwarding requires an attached tunnel"
        );
    }
    if !spec.volumes.is_empty() {
        bail!("tak docker run does not support volume mounts yet");
    }
    if spec.name.is_some() {
        bail!("tak docker run does not support --name yet");
    }
    if spec.cpus.is_some() || spec.memory.is_some() {
        bail!("tak docker run does not support resource limits yet");
    }
    let _rm_is_always_effective = spec.rm;
    Ok(())
}
