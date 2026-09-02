use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};
use tak_proto::local_daemon::v2::{Operation, OutputArtifact, Response};

#[path = "outputs/download.rs"]
mod download;
#[path = "outputs/exchange.rs"]
mod exchange;
#[path = "outputs/materialize.rs"]
mod materialize;

use exchange::Policy;

pub(super) async fn retrieve(socket: &Path, run_id: &str, destination: &Path) -> Result<()> {
    let bundle = fetch(socket, run_id).await?;
    write_fresh(socket, destination, &bundle).await
}

pub(crate) struct ValidatedOutputBundle {
    pub(crate) artifacts: Vec<OutputArtifact>,
    pub(crate) manifest: WorkspaceManifest,
}

pub(crate) async fn fetch(socket: &Path, run_id: &str) -> Result<ValidatedOutputBundle> {
    fetch_with_policy(socket, run_id, Policy::Management).await
}

pub(crate) async fn fetch_foreground(socket: &Path, run_id: &str) -> Result<ValidatedOutputBundle> {
    fetch_with_policy(socket, run_id, Policy::Foreground).await
}

async fn fetch_with_policy(
    socket: &Path,
    run_id: &str,
    policy: Policy,
) -> Result<ValidatedOutputBundle> {
    let response = policy
        .response(
            socket,
            "tak-runs-outputs",
            Operation::GetOutputManifest {
                run_id: run_id.to_owned(),
            },
        )
        .await?;
    validated_bundle(run_id, response)
}

fn validated_bundle(run_id: &str, response: Response) -> Result<ValidatedOutputBundle> {
    let Response::OutputManifest {
        run_id: response_run,
        expired,
        artifacts,
        ..
    } = response
    else {
        bail!(super::MISMATCH_DIAGNOSTIC)
    };
    if response_run != run_id {
        bail!(super::MISMATCH_DIAGNOSTIC);
    }
    if expired {
        bail!("Run output artifacts have expired.");
    }
    let manifest = validate(&artifacts)?;
    Ok(ValidatedOutputBundle {
        artifacts,
        manifest,
    })
}

pub(crate) async fn write_fresh(
    socket: &Path,
    destination: &Path,
    bundle: &ValidatedOutputBundle,
) -> Result<()> {
    write_fresh_with_policy(socket, destination, bundle, Policy::Management).await
}

pub(crate) async fn write_fresh_foreground(
    socket: &Path,
    destination: &Path,
    bundle: &ValidatedOutputBundle,
) -> Result<()> {
    write_fresh_with_policy(socket, destination, bundle, Policy::Foreground).await
}

async fn write_fresh_with_policy(
    socket: &Path,
    destination: &Path,
    bundle: &ValidatedOutputBundle,
    policy: Policy,
) -> Result<()> {
    if fs::symlink_metadata(destination).is_ok() {
        bail!("run output destination already exists");
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("create run output destination parent {}", parent.display())
        })?;
    }
    fs::create_dir(destination)
        .with_context(|| format!("create run output destination {}", destination.display()))?;
    let mut fresh = FreshDestination::new(destination);
    materialize::all(socket, destination, &bundle.artifacts, policy).await?;
    fresh.keep();
    Ok(())
}

fn validate(artifacts: &[OutputArtifact]) -> Result<WorkspaceManifest> {
    let mut entries = Vec::new();
    for artifact in artifacts {
        safe_path(&artifact.path)?;
        entries.push(validate_metadata(artifact)?);
    }
    WorkspaceManifest::new(entries).map_err(|_| anyhow::anyhow!("daemon output manifest is unsafe"))
}

fn validate_metadata(artifact: &OutputArtifact) -> Result<WorkspaceEntry> {
    let expected = match artifact.entry_type.as_str() {
        "file" if artifact.symlink_target.is_none() => WorkspaceEntry::file(
            &artifact.path,
            artifact.executable,
            artifact.size,
            &artifact.sha256,
        ),
        "directory" if artifact.symlink_target.is_none() => {
            WorkspaceEntry::directory(&artifact.path)
        }
        "symlink" if !artifact.executable => WorkspaceEntry::symlink(
            &artifact.path,
            artifact
                .symlink_target
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("daemon output symlink has no target"))?,
        ),
        "file" | "directory" | "symlink" => {
            bail!("daemon output manifest contains invalid entry metadata")
        }
        _ => bail!("daemon output manifest contains an unsupported entry type"),
    }
    .map_err(|_| anyhow::anyhow!("daemon output manifest contains unsafe metadata"))?;
    if expected.executable != artifact.executable
        || expected.size != artifact.size
        || expected.content_sha256 != artifact.sha256
    {
        bail!("daemon output manifest contains invalid entry metadata");
    }
    Ok(expected)
}

fn safe_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("daemon output path is unsafe");
    }
    Ok(())
}

struct FreshDestination {
    path: PathBuf,
    keep: bool,
}

impl FreshDestination {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            keep: false,
        }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for FreshDestination {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
