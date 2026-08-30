use std::path::PathBuf;

use anyhow::{Result, bail};
use tak_loader::V2AuthoredRoot;

use super::run_command::RunCliArgs;

mod resolve;
mod submission;
mod workspace;

pub(super) async fn execute(root: V2AuthoredRoot, args: RunCliArgs) -> Result<()> {
    validate_overrides(&args)?;
    let workspace = workspace::build(&root.workspace_root)?;
    let submission = resolve::resolve(&root, &args, workspace.descriptor.clone())?;
    submission::submit_and_attach(socket_path(), submission, workspace.archive).await
}

fn socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(tak_core::runtime_paths::default_daemon_socket_path)
}

fn validate_overrides(args: &RunCliArgs) -> Result<()> {
    if args.local
        || args.local_no_container
        || args.remote
        || args.container
        || args.container_image.is_some()
        || args.container_dockerfile.is_some()
        || args.container_build_context.is_some()
    {
        bail!("v2 execution overrides are not active in this build")
    }
    Ok(())
}
