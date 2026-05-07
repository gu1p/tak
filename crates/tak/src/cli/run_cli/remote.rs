use anyhow::{Result, bail};

use crate::cli::command_model::{RemoteCommands, RemoteTaskCommands};
use crate::cli::remote_add::{StartMode, run_remote_add};
use crate::cli::remote_inventory::{add_remote, list_remotes, remove_remote};
use crate::cli::remote_logs::run_remote_logs;
use crate::cli::remote_scan::run_remote_scan;
use crate::cli::remote_status::run_remote_status;
use crate::cli::remote_tasks::{run_remote_task_logs, run_remote_tasks};

pub(super) async fn run_remote_command(command: RemoteCommands) -> Result<()> {
    match command {
        RemoteCommands::Add { token, words } => {
            if token.is_none() && words.is_none() {
                run_remote_add(StartMode::Menu).await?;
            } else if words.as_ref().is_some_and(|values| values.is_empty()) {
                run_remote_add(StartMode::Words).await?;
            } else {
                let token = resolve_remote_add_token(token, words.as_deref())?;
                let remote = add_remote(&token).await?;
                println!("added remote {}", remote.node_id);
            }
        }
        RemoteCommands::Scan => {
            run_remote_scan().await?;
        }
        RemoteCommands::List => {
            for remote in list_remotes()? {
                println!(
                    "{} {} pools={} tags={} capabilities={} enabled={}",
                    remote.node_id,
                    remote.base_url,
                    remote.pools.join(","),
                    remote.tags.join(","),
                    remote.capabilities.join(","),
                    remote.enabled
                );
            }
        }
        RemoteCommands::Remove { node_id } => {
            if remove_remote(&node_id)? {
                println!("removed remote {node_id}");
            } else {
                println!("remote not found: {node_id}");
            }
        }
        RemoteCommands::Status {
            node_ids,
            watch,
            interval_ms,
        } => {
            run_remote_status(&node_ids, watch, interval_ms).await?;
        }
        RemoteCommands::Logs {
            node_id,
            all,
            lines,
        } => {
            run_remote_logs(&node_id, all, lines).await?;
        }
        RemoteCommands::Tasks {
            node_id,
            active,
            limit,
        } => {
            run_remote_tasks(&node_id, active, limit).await?;
        }
        RemoteCommands::Task { command } => run_remote_task_command(command).await?,
    }
    Ok(())
}

async fn run_remote_task_command(command: RemoteTaskCommands) -> Result<()> {
    match command {
        RemoteTaskCommands::Logs {
            node_id,
            task_run_id,
            attempt,
            follow,
            interval_ms,
        } => {
            run_remote_task_logs(&node_id, &task_run_id, attempt, follow, interval_ms).await?;
        }
    }
    Ok(())
}

fn resolve_remote_add_token(token: Option<String>, words: Option<&[String]>) -> Result<String> {
    if let Some(token) = token {
        return Ok(token);
    }

    let phrase = words
        .unwrap_or_default()
        .iter()
        .flat_map(|value| value.split_whitespace())
        .collect::<Vec<_>>();
    if phrase.is_empty() {
        bail!("remote add requires a token or `--words`");
    }

    tak_proto::decode_tor_invite_words(&phrase.join(" "))
}
