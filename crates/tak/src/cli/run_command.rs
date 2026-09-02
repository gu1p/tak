use super::*;

pub(super) struct RunCliArgs {
    pub(super) labels: Vec<String>,
    pub(super) jobs: usize,
    pub(super) keep_going: bool,
    pub(super) pass_env: Vec<String>,
    pub(super) local: bool,
    pub(super) local_no_container: bool,
    pub(super) remote: bool,
    pub(super) container: bool,
    pub(super) container_image: Option<String>,
    pub(super) container_dockerfile: Option<String>,
    pub(super) container_build_context: Option<String>,
}

pub(super) async fn run_task_command(args: RunCliArgs) -> Result<std::process::ExitCode> {
    if args.labels.is_empty() {
        bail!("run requires at least one label");
    }
    let cwd = std::env::current_dir().context("resolve current directory")?;
    let root = tak_loader::inspect_authored_root_module(&cwd, &LoadOptions::default())?;
    super::daemon_run::execute(root, args).await
}
