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
    Run {
        labels: Vec<String>,
        #[arg(short = 'j', long = "jobs", default_value_t = 1)]
        jobs: usize,
        #[arg(long = "keep-going", default_value_t = false)]
        keep_going: bool,
    },
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    Status,
}

#[derive(Debug, Subcommand)]
pub(super) enum RemoteCommands {
    Add { token: String },
    List,
    Remove { node_id: String },
}
