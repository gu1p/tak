use super::*;
use std::process::ExitCode;

mod remote;
use remote::run_remote_command;

/// Parses CLI input and dispatches Tak commands.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_cli() -> Result<ExitCode> {
    tak_core::crypto_provider::ensure_rustls_crypto_provider();
    let cli = Cli::parse();
    let selectors = DockerCliSelectors {
        local: cli.local,
        node: cli.node,
        arch: cli.arch,
        os: cli.os,
        pool: cli.pool,
        tags: cli.tags,
        capabilities: cli.capabilities,
        transport: cli.transport,
    };

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
            if task.doc.trim().is_empty() {
                println!("doc: (none)");
            } else {
                println!("doc:");
                for line in task.doc.trim().lines() {
                    println!("  {line}");
                }
            }
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
        Commands::Exec {
            cwd,
            env,
            local,
            local_no_container,
            remote,
            container,
            container_image,
            container_dockerfile,
            container_build_context,
            argv,
        } => {
            return run_exec_command(ExecCliArgs {
                cwd,
                env,
                local,
                local_no_container,
                remote,
                container,
                container_image,
                container_dockerfile,
                container_build_context,
                argv,
            })
            .await;
        }
        Commands::Run {
            labels,
            jobs,
            keep_going,
            local,
            local_no_container,
            remote,
            container,
            container_image,
            container_dockerfile,
            container_build_context,
        } => {
            run_task_command(RunCliArgs {
                labels,
                jobs,
                keep_going,
                local,
                local_no_container,
                remote,
                container,
                container_image,
                container_dockerfile,
                container_build_context,
            })
            .await?;
        }
        Commands::Docker { argv } => {
            return run_docker_command(selectors, argv).await;
        }
        Commands::Remote { command } => run_remote_command(command).await?,
        Commands::Local { command } => match command {
            super::command_model::LocalCommands::Status { watch, interval_ms } => {
                run_local_status(watch, interval_ms).await?;
            }
        },
        Commands::Task { command } => match command {
            super::command_model::TaskCommands::List { limit } => {
                print_task_history(limit)?;
            }
            super::command_model::TaskCommands::Logs {
                task_run_id,
                follow,
                interval_ms,
            } => {
                print_task_logs(&task_run_id, follow, interval_ms).await?;
            }
        },
        Commands::Status {
            node_ids,
            watch,
            interval_ms,
        } => {
            run_status(&node_ids, watch, interval_ms).await?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
