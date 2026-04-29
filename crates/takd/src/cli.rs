use anyhow::Result;
use clap::Parser;

use crate::logging::{init_service_logging, read_service_log_tail};
use crate::qr_render::render_onboarding_view;
use crate::serve_lock::ServiceStateLock;
use crate::word_table::render_words_table_view;
use command_model::{Cli, Commands, TokenCommands};
use tak_proto::encode_tor_invite_words;
use takd::agent::{
    InitAgentOptions, default_config_root, default_state_root, init_agent, read_config, read_token,
    read_token_wait,
};
use takd::serve_agent;

mod command_model;
mod status_output;

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
            image_cache_budget_percent,
            image_cache_budget_gb,
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
                    image_cache_budget_percent,
                    image_cache_budget_gb,
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
            let _serve_lock = ServiceStateLock::acquire(&state_root)?;
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
                words_table,
            } => {
                let state_root = state_root.unwrap_or(default_state_root()?);
                let token = if wait {
                    read_token_wait(&state_root, timeout_secs)?
                } else {
                    read_token(&state_root)?
                };
                if qr {
                    print!("{}", render_onboarding_view(&token)?);
                } else if words_table {
                    print!("{}", render_words_table_view(&token)?);
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
