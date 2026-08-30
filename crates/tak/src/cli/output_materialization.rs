use std::path::Path;

use anyhow::{Context, Result, ensure};
use tak_core::v2::WorkspaceManifest;

mod apply;
mod preflight;
mod snapshot;

#[cfg(test)]
#[path = "output_materialization/preflight_tests.rs"]
mod preflight_tests;

fn preflight(
    workspace_root: &Path,
    submitted: &WorkspaceManifest,
    outputs: &WorkspaceManifest,
) -> Result<()> {
    preflight::check(workspace_root, submitted, outputs)
}

pub(super) async fn materialize(
    socket: &Path,
    run_id: &str,
    context: &super::run_checkout_store::CheckoutContext,
) -> Result<()> {
    let bundle = super::runs_cli::outputs::fetch(socket, run_id).await?;
    if bundle.manifest.entries.is_empty() {
        return Ok(());
    }
    ensure!(
        std::fs::symlink_metadata(&context.root)?.is_dir(),
        "original checkout is no longer a directory"
    );
    let staging = Staging::new(&context.root);
    super::runs_cli::outputs::write_fresh(socket, staging.path(), &bundle).await?;
    preflight(
        &context.root,
        &context.submitted_manifest,
        &bundle.manifest,
    )
    .with_context(|| {
        format!(
            "run {run_id} outputs remain in takd; retrieve them with `tak runs outputs {run_id} --to DIR`"
        )
    })?;
    apply::all(&context.root, staging.path(), &bundle.manifest)
}

struct Staging {
    path: std::path::PathBuf,
}

impl Staging {
    fn new(checkout: &Path) -> Self {
        Self {
            path: checkout.join(format!(".tak-output-stage-{}", uuid::Uuid::new_v4())),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if std::fs::symlink_metadata(&self.path).is_ok() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
