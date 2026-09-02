use std::fs;
use std::path::Path;

use anyhow::Result;
use tak_proto::local_daemon::v2::OutputArtifact;

use super::exchange::Policy;

pub(super) async fn all(
    socket: &Path,
    root: &Path,
    artifacts: &[OutputArtifact],
    policy: Policy,
) -> Result<()> {
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.entry_type == "directory")
    {
        fs::create_dir_all(root.join(&artifact.path))?;
    }
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.entry_type == "file")
    {
        let destination = root.join(&artifact.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        super::download::file(socket, artifact, &destination, policy).await?;
    }
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.entry_type == "symlink")
    {
        let destination = root.join(&artifact.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        symlink(&destination, artifact)?;
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(destination: &Path, artifact: &OutputArtifact) -> Result<()> {
    use std::os::unix::fs::symlink;

    symlink(
        artifact
            .symlink_target
            .as_deref()
            .expect("preflight validated symlink target"),
        destination,
    )?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink(_destination: &Path, _artifact: &OutputArtifact) -> Result<()> {
    anyhow::bail!("output symlinks are not supported on this platform")
}
