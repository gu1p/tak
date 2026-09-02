use std::process::ExitCode;

use super::*;

pub(super) mod resolve;
mod runtime;

pub(super) struct ExecCliArgs {
    pub(super) cwd: Option<String>,
    pub(super) env: Vec<String>,
    pub(super) pass_env: Vec<String>,
    pub(super) local: bool,
    pub(super) local_no_container: bool,
    pub(super) remote: bool,
    pub(super) container: bool,
    pub(super) container_image: Option<String>,
    pub(super) container_dockerfile: Option<String>,
    pub(super) container_build_context: Option<String>,
    pub(super) argv: Vec<String>,
}

pub(super) async fn run_exec_command(args: ExecCliArgs) -> Result<ExitCode> {
    let root = std::env::current_dir().context("failed to resolve current directory")?;
    if args.remote && args.container {
        eprintln!(
            "warning: --container is redundant with --remote; remote execution already implies a container"
        );
    }
    let workspace = super::daemon_run::build_workspace(&root)?;
    let submission = resolve::submission(&args, workspace.descriptor.clone()).await?;
    super::daemon_run::submit_resolved_exit_code(&root, submission, workspace).await
}
