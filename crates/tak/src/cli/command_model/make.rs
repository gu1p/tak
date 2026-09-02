#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(in crate::cli) enum MakeParallelOutputArg {
    Live,
    Grouped,
}

#[derive(Debug, clap::Args)]
pub(in crate::cli) struct MakeArgs {
    /// The Make goal to execute.
    pub(in crate::cli) goal: String,
    /// Pass one named client environment variable to the Make run.
    #[arg(long = "pass-env", value_name = "NAME")]
    pub(in crate::cli) pass_env: Vec<String>,
    /// Force local placement; an annotated container runtime may still be used.
    #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
    pub(in crate::cli) local: bool,
    /// Force local host execution without a container.
    #[arg(long = "local-no-container", default_value_t = false)]
    pub(in crate::cli) local_no_container: bool,
    /// Force remote containerized execution.
    #[arg(long = "remote", default_value_t = false)]
    pub(in crate::cli) remote: bool,
    /// Force a local container. With `--remote`, accepted as a compatibility alias.
    #[arg(long = "container", default_value_t = false)]
    pub(in crate::cli) container: bool,
    /// Use this container image for execution.
    #[arg(long = "container-image")]
    pub(in crate::cli) container_image: Option<String>,
    /// Build a container from this Dockerfile.
    #[arg(long = "container-dockerfile")]
    pub(in crate::cli) container_dockerfile: Option<String>,
    /// Override the Dockerfile build context directory.
    #[arg(long = "container-build-context")]
    pub(in crate::cli) container_build_context: Option<String>,
    /// Present concurrent goal output live or grouped by completed goal.
    #[arg(long = "parallel-output", value_enum)]
    pub(in crate::cli) parallel_output: Option<MakeParallelOutputArg>,
}
