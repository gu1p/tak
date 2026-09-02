use std::process::ExitCode;

use anyhow::{Context, Result};

use super::DockerCliSelectors;
use super::run_spec::parse_docker_run;
use super::run_validate::validate_docker_run_spec;

mod resolved;

pub(super) async fn run_docker_run(
    selectors: DockerCliSelectors,
    args: &[String],
) -> Result<ExitCode> {
    let spec = parse_docker_run(args)?;
    validate_docker_run_spec(&spec)?;
    let root = std::env::current_dir().context("failed to resolve current directory")?;
    let workspace = super::super::daemon_run::build_workspace(&root)?;
    let submission = resolved::submission(&selectors, &spec, workspace.descriptor.clone()).await?;
    super::super::daemon_run::submit_resolved(&root, submission, workspace).await?;
    Ok(ExitCode::SUCCESS)
}
