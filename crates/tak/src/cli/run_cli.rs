use super::*;
use std::sync::Arc;

/// Parses CLI input and dispatches Tak commands.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_cli() -> Result<()> {
    tak_core::crypto_provider::ensure_rustls_crypto_provider();
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", crate::list_tui::render_list(&spec));
        }
        Commands::Tree => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", crate::list_tui::render_tree(&spec)?);
        }
        Commands::Docs { command } => match command {
            super::command_model::DocsCommands::Dump => {
                print!("{}", crate::docs::render_docs_dump()?);
            }
        },
        Commands::Explain { label } => {
            let spec = load_workspace_from_cwd()?;
            let label = parse_input_label(&spec, &label, "explain")?;
            let task = spec
                .tasks
                .get(&label)
                .ok_or_else(|| anyhow!("task not found: {label}"))?;

            println!("label: {}", canonical_label(&label));
            if task.deps.is_empty() {
                println!("deps: (none)");
            } else {
                println!("deps:");
                for dep in &task.deps {
                    println!("  - {}", canonical_label(dep));
                }
            }
            println!("steps: {}", task.steps.len());
            println!("needs: {}", task.needs.len());
            println!(
                "timeout_s: {}",
                task.timeout_s
                    .map_or_else(|| "none".to_string(), |s| s.to_string())
            );
            println!("retry_attempts: {}", task.retry.attempts);
        }
        Commands::Graph { label, format } => {
            let spec = load_workspace_from_cwd()?;
            if format != "dot" {
                bail!("unsupported format: {format}");
            }
            let scope = match label {
                Some(value) => vec![parse_input_label(&spec, &value, "graph")?],
                None => spec.tasks.keys().cloned().collect(),
            };
            print_dot_graph(&spec, &scope);
        }
        Commands::Web { label } => {
            let spec = load_workspace_from_cwd()?;
            let parsed = label
                .as_deref()
                .map(|value| parse_input_label(&spec, value, "web"))
                .transpose()
                .context("failed to parse optional web graph label")?;
            crate::web::serve_graph_ui(&spec, parsed.as_ref()).await?;
        }
        Commands::Run {
            labels,
            jobs,
            keep_going,
            local,
            remote,
            container,
            container_image,
            container_dockerfile,
            container_build_context,
        } => {
            if labels.is_empty() {
                bail!("run requires at least one label");
            }

            let spec = load_workspace_from_cwd()?;
            let targets = labels
                .iter()
                .map(|label| parse_input_label(&spec, label, "run"))
                .collect::<Result<Vec<_>>>()?;
            let spec = apply_run_execution_overrides(
                &spec,
                &targets,
                RunExecutionOverrideArgs {
                    local,
                    remote,
                    container,
                    container_image: container_image.as_deref(),
                    container_dockerfile: container_dockerfile.as_deref(),
                    container_build_context: container_build_context.as_deref(),
                },
            )?;

            let summary = run_tasks(
                &spec,
                &targets,
                &RunOptions {
                    jobs,
                    keep_going,
                    lease_socket: std::env::var_os("TAKD_SOCKET").map(std::path::PathBuf::from),
                    lease_ttl_ms: 30_000,
                    lease_poll_interval_ms: 200,
                    session_id: std::env::var("TAK_SESSION_ID").ok(),
                    user: std::env::var("TAK_USER").ok(),
                    output_observer: Some(Arc::new(StdStreamOutputObserver::default())),
                },
            )
            .await?;

            for (label, result) in summary.results {
                println!(
                    "{}: {} (attempts={}, exit_code={}, placement={}, remote_node={}, transport={}, reason={}, context_hash={}, runtime={}, runtime_engine={})",
                    canonical_label(&label),
                    if result.success { "ok" } else { "failed" },
                    result.attempts,
                    result
                        .exit_code
                        .map_or_else(|| "none".to_string(), |code| code.to_string()),
                    result.placement_mode.as_str(),
                    result.remote_node_id.as_deref().unwrap_or("none"),
                    result.remote_transport_kind.as_deref().unwrap_or("none"),
                    result.decision_reason.as_deref().unwrap_or("none"),
                    result.context_manifest_hash.as_deref().unwrap_or("none"),
                    result.remote_runtime_kind.as_deref().unwrap_or("none"),
                    result.remote_runtime_engine.as_deref().unwrap_or("none")
                );
            }
        }
        Commands::Remote { command } => match command {
            super::command_model::RemoteCommands::Add { token } => {
                let remote = add_remote(&token).await?;
                println!("added remote {}", remote.node_id);
            }
            super::command_model::RemoteCommands::Scan => {
                run_remote_scan().await?;
            }
            super::command_model::RemoteCommands::List => {
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
            super::command_model::RemoteCommands::Remove { node_id } => {
                if remove_remote(&node_id)? {
                    println!("removed remote {node_id}");
                } else {
                    println!("remote not found: {node_id}");
                }
            }
            super::command_model::RemoteCommands::Status {
                node_ids,
                watch,
                interval_ms,
            } => {
                run_remote_status(&node_ids, watch, interval_ms).await?;
            }
        },
        Commands::Status => {
            bail!("coordination status is unavailable in this client-only build");
        }
    }

    Ok(())
}
