#[derive(Debug, Parser)]
#[command(name = "tak")]
#[command(about = "Tak task orchestrator")]
#[command(version = env!("TAK_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
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
    Status,
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommands {
    Start,
    Status,
}
