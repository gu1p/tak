use super::*;

#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(about = "Tak task orchestrator")]
#[command(version = env!("TAK_VERSION"))]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    List,
    Tree,
    Docs {
        #[command(subcommand)]
        command: DocsCommands,
    },
    Explain {
        label: String,
    },
    Graph {
        label: Option<String>,
        #[arg(long, default_value = "dot")]
        format: String,
    },
    Web {
        label: Option<String>,
    },
    Exec {
        #[arg(long = "cwd")]
        cwd: Option<String>,
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        #[arg(long = "remote", default_value_t = false)]
        remote: bool,
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        #[arg(long = "container-image")]
        container_image: Option<String>,
        #[arg(long = "container-dockerfile")]
        container_dockerfile: Option<String>,
        #[arg(long = "container-build-context")]
        container_build_context: Option<String>,
        #[arg(last = true, required = true, num_args = 1.., allow_hyphen_values = true)]
        argv: Vec<String>,
    },
    Run {
        labels: Vec<String>,
        #[arg(short = 'j', long = "jobs", default_value_t = 1)]
        jobs: usize,
        #[arg(long = "keep-going", default_value_t = false)]
        keep_going: bool,
        #[arg(long = "local", default_value_t = false, conflicts_with = "remote")]
        local: bool,
        #[arg(long = "remote", default_value_t = false)]
        remote: bool,
        #[arg(long = "container", default_value_t = false)]
        container: bool,
        #[arg(long = "container-image")]
        container_image: Option<String>,
        #[arg(long = "container-dockerfile")]
        container_dockerfile: Option<String>,
        #[arg(long = "container-build-context")]
        container_build_context: Option<String>,
    },
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    Status,
}

#[derive(Debug, Subcommand)]
pub(super) enum DocsCommands {
    Dump,
}

#[derive(Debug, Subcommand)]
pub(super) enum RemoteCommands {
    Add {
        token: String,
    },
    Scan,
    List,
    Remove {
        node_id: String,
    },
    Status {
        #[arg(long = "node")]
        node_ids: Vec<String>,
        #[arg(long, default_value_t = false)]
        watch: bool,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
}
