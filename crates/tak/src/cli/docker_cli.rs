use std::process::ExitCode;

use anyhow::{Result, bail};

mod ps;
mod run;
mod run_spec;
mod run_validate;
mod selectors;

#[derive(Debug)]
pub(super) struct DockerCliSelectors {
    pub(super) local: bool,
    pub(super) node: Option<String>,
    pub(super) arch: Option<String>,
    pub(super) os: Option<String>,
    pub(super) pool: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) capabilities: Vec<String>,
    pub(super) transport: Option<String>,
}

pub(super) async fn run_docker_command(
    selectors: DockerCliSelectors,
    argv: Vec<String>,
) -> Result<ExitCode> {
    let Some((command, rest)) = argv.split_first() else {
        bail!("tak docker requires a Docker subcommand");
    };

    match command.as_str() {
        "build" => bail!(
            "tak docker build is not supported. Tak executes containers and does not guarantee Docker image state across invocations; use `tak docker run -f Dockerfile --build-context . ...` for a per-run Dockerfile build."
        ),
        "ps" => ps::run_docker_ps(selectors, rest).await,
        "run" => run::run_docker_run(selectors, rest).await,
        other => bail!("tak docker {other} is not supported yet; supported subcommands: run, ps"),
    }
}
