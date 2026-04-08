use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use takd::agent::{
    InitAgentOptions, default_config_root, default_state_root, init_agent, read_config, read_token,
    read_token_wait,
};
use takd::serve_agent;

#[derive(Debug, Parser)]
#[command(name = "takd")]
#[command(about = "Tak execution agent")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
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
    },
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
}

#[derive(Debug, Subcommand)]
enum TokenCommands {
    Show {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        wait: bool,
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
    },
}

pub async fn run_cli() -> Result<()> {
    match Cli::parse().command {
        Commands::Init {
            config_root,
            state_root,
            node_id,
            display_name,
            transport,
            base_url,
            pools,
            tags,
            capabilities,
        } => {
            let config_root = config_root.unwrap_or(default_config_root()?);
            let state_root = state_root.unwrap_or(default_state_root()?);
            init_agent(
                &config_root,
                &state_root,
                InitAgentOptions {
                    node_id: node_id.as_deref(),
                    display_name: display_name.as_deref(),
                    transport: transport.as_deref(),
                    base_url: base_url.as_deref(),
                    pools: &pools,
                    tags: &tags,
                    capabilities: &capabilities,
                },
            )?;
        }
        Commands::Serve {
            config_root,
            state_root,
        } => {
            let config_root = config_root.unwrap_or(default_config_root()?);
            let state_root = state_root.unwrap_or(default_state_root()?);
            serve_agent(&config_root, &state_root).await?;
        }
        Commands::Status { config_root } => {
            let config = read_config(&config_root.unwrap_or(default_config_root()?))?;
            println!("node_id: {}", config.node_id);
            println!("transport: {}", config.transport);
            println!(
                "readiness: {}",
                if config.base_url.is_some() {
                    "advertised"
                } else {
                    "pending"
                }
            );
            if let Some(base_url) = config.base_url {
                println!("reachability: unverified");
                println!("base_url: {base_url}");
            }
        }
        Commands::Token { command } => match command {
            TokenCommands::Show {
                state_root,
                wait,
                timeout_secs,
            } => {
                let state_root = state_root.unwrap_or(default_state_root()?);
                println!(
                    "{}",
                    if wait {
                        read_token_wait(&state_root, timeout_secs)?
                    } else {
                        read_token(&state_root)?
                    }
                );
            }
        },
    }

    Ok(())
}
