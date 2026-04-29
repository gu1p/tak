use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "takd")]
#[command(about = "Tak execution agent")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    Init {
        #[arg(long)]
        config_root: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        node_id: Option<String>,
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long)]
        transport: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long = "pool")]
        pools: Vec<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "capability")]
        capabilities: Vec<String>,
        #[arg(long)]
        image_cache_budget_percent: Option<f64>,
        #[arg(long)]
        image_cache_budget_gb: Option<f64>,
    },
    Serve {
        #[arg(long)]
        config_root: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        config_root: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    Logs {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = 200)]
        lines: usize,
    },
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum TokenCommands {
    Show {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        wait: bool,
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
        #[arg(long, default_value_t = false)]
        qr: bool,
        #[arg(long, default_value_t = false, conflicts_with_all = ["qr", "words_table"])]
        words: bool,
        #[arg(long, default_value_t = false, conflicts_with_all = ["qr", "words"])]
        words_table: bool,
    },
}
