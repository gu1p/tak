use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::logging::{init_service_logging, read_service_log_tail};
use crate::qr_render::render_onboarding_view;
use tak_proto::encode_tor_invite_words;
use takd::agent::{
    InitAgentOptions, default_config_root, default_state_root, init_agent, read_config, read_token,
    read_token_wait,
};
use takd::serve_agent;

mod status_output;

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
enum TokenCommands {
    Show {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        wait: bool,
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
        #[arg(long, default_value_t = false)]
        qr: bool,
        #[arg(long, default_value_t = false, conflicts_with = "qr")]
        words: bool,
    },
}

pub async fn run_cli() -> Result<()> {
    tak_core::crypto_provider::ensure_rustls_crypto_provider();
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
            init_service_logging(&state_root)?;
            if let Err(err) = serve_agent(&config_root, &state_root).await {
                tracing::error!("takd serve failed: {err:#}");
                return Err(err);
            }
        }
        Commands::Status {
            config_root,
            state_root,
        } => {
            let config = read_config(&config_root.unwrap_or(default_config_root()?))?;
            let state_root = state_root.unwrap_or(default_state_root()?);
            status_output::print_status(&config, &state_root)?;
        }
        Commands::Logs { state_root, lines } => {
            let state_root = state_root.unwrap_or(default_state_root()?);
            print!("{}", read_service_log_tail(&state_root, lines)?);
        }
        Commands::Token { command } => match command {
            TokenCommands::Show {
                state_root,
                wait,
                timeout_secs,
                qr,
                words,
            } => {
                let state_root = state_root.unwrap_or(default_state_root()?);
                let token = if wait {
                    read_token_wait(&state_root, timeout_secs)?
                } else {
                    read_token(&state_root)?
                };
                if qr {
                    print!("{}", render_onboarding_view(&token)?);
                } else if words {
                    println!("{}", encode_tor_invite_words(&token)?);
                } else {
                    println!("{token}");
                }
            }
        },
    }

    Ok(())
}
