use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_core::v2::{PlacementCandidate, RemoteRequirements, RunSubmission};
use tak_loader::V2AuthoredRoot;
use tak_proto::local_daemon::v2::{Request, Response, RunEvent};

use super::run_command::RunCliArgs;

mod overrides;
#[cfg(test)]
mod overrides_tests;
mod resolve;
mod submission;
mod workspace;
#[cfg(test)]
mod workspace_context_tests;
pub(super) use workspace::WorkspaceBundle;

pub(super) trait PersistedEventRenderer: Send + Sync {
    fn render(&self, event: &RunEvent) -> Result<bool>;
}

pub(super) async fn foreground_response(socket_path: &Path, request: &Request) -> Result<Response> {
    submission::foreground_response(socket_path, request).await
}

pub(super) async fn execute(root: V2AuthoredRoot, args: RunCliArgs) -> Result<ExitCode> {
    let execution_override = overrides::resolve(&args)?;
    if args.remote && args.container {
        eprintln!(
            "warning: --container is redundant with --remote; remote execution already implies a container"
        );
    }
    let contexts = resolve::workspace_contexts(&root, &args)?;
    let context_refs = contexts.iter().collect::<Vec<_>>();
    let workspace = workspace::build_for_contexts(&root.workspace_root, &context_refs)?;
    let checkout = super::run_checkout_store::CheckoutContext::new(
        &root.workspace_root,
        workspace.descriptor.manifest.clone(),
    )?;
    let socket_path = socket_path();
    let submission = resolve::resolve(
        &root,
        &args,
        workspace.descriptor.clone(),
        &workspace.gitignored_paths,
        &socket_path,
        execution_override.as_ref(),
    )
    .await?;
    submission::submit_and_attach(socket_path, submission, workspace.archive, checkout, None).await
}

pub(super) fn build_workspace(root: &Path) -> Result<WorkspaceBundle> {
    workspace::build(root)
}

pub(super) async fn remote_candidates(
    requirements: RemoteRequirements,
) -> Result<Vec<PlacementCandidate>> {
    submission::remote_candidates(&socket_path(), requirements).await
}

pub(super) async fn submit_resolved(
    root: &Path,
    submission: RunSubmission,
    workspace: WorkspaceBundle,
) -> Result<()> {
    let status = submit_resolved_exit_code(root, submission, workspace).await?;
    if status != ExitCode::SUCCESS {
        bail!("daemon-owned run did not succeed")
    }
    Ok(())
}

pub(super) async fn submit_resolved_exit_code(
    root: &Path,
    submission: RunSubmission,
    workspace: WorkspaceBundle,
) -> Result<ExitCode> {
    let checkout = super::run_checkout_store::CheckoutContext::new(
        root,
        workspace.descriptor.manifest.clone(),
    )?;
    submission::submit_and_attach(socket_path(), submission, workspace.archive, checkout, None)
        .await
}

pub(super) async fn submit_resolved_exit_code_with_renderer(
    root: &Path,
    submission: RunSubmission,
    workspace: WorkspaceBundle,
    renderer: &dyn PersistedEventRenderer,
) -> Result<ExitCode> {
    let checkout = super::run_checkout_store::CheckoutContext::new(
        root,
        workspace.descriptor.manifest.clone(),
    )?;
    submission::submit_and_attach(
        socket_path(),
        submission,
        workspace.archive,
        checkout,
        Some(renderer),
    )
    .await
}

fn socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}
