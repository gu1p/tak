/// Parses CLI input and dispatches Tak commands.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", list_tui::render_list(&spec));
        }
        Commands::Tree => {
            let spec = load_workspace_from_cwd()?;
            print!("{}", list_tui::render_tree(&spec)?);
        }
        Commands::Explain { label } => {
            let spec = load_workspace_from_cwd()?;
            let label = parse_input_label(&label)?;
            let task = spec
                .tasks
                .get(&label)
                .ok_or_else(|| anyhow!("task not found: {label}"))?;

            println!("label: {label}");
            if task.deps.is_empty() {
                println!("deps: (none)");
            } else {
                println!("deps:");
                for dep in &task.deps {
                    println!("  - {dep}");
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
                Some(value) => vec![parse_input_label(&value)?],
                None => spec.tasks.keys().cloned().collect(),
            };
            print_dot_graph(&spec, &scope);
        }
        Commands::Web { label } => {
            let spec = load_workspace_from_cwd()?;
            let parsed = label
                .as_deref()
                .map(parse_input_label)
                .transpose()
                .context("failed to parse optional web graph label")?;
            web::serve_graph_ui(&spec, parsed.as_ref()).await?;
        }
        Commands::Run {
            labels,
            jobs,
            keep_going,
        } => {
            if labels.is_empty() {
                bail!("run requires at least one label");
            }

            let targets = labels
                .iter()
                .map(|label| parse_input_label(label))
                .collect::<Result<Vec<_>>>()?;

            let socket_path = resolve_daemon_socket_path();
            let workspace_root =
                std::env::current_dir().context("failed to resolve current directory")?;
            let ran_via_daemon = try_run_via_daemon(
                socket_path.clone(),
                workspace_root,
                &targets,
                jobs,
                keep_going,
            )
            .await?;
            if ran_via_daemon {
                return Ok(());
            }

            let spec = load_workspace_from_cwd()?;

            let summary = run_tasks(
                &spec,
                &targets,
                &RunOptions {
                    jobs,
                    keep_going,
                    lease_socket: Some(socket_path),
                    lease_ttl_ms: env_u64("TAK_LEASE_TTL_MS", 30_000),
                    lease_poll_interval_ms: env_u64("TAK_LEASE_POLL_MS", 200),
                    session_id: std::env::var("TAK_SESSION_ID").ok(),
                    user: std::env::var("TAK_USER").ok(),
                },
            )
            .await?;

            for (label, result) in summary.results {
                println!(
                    "{label}: {} (attempts={}, exit_code={}, placement={}, remote_node={}, transport={}, reason={}, context_hash={}, runtime={}, runtime_engine={})",
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
        Commands::Status => {
            let snapshot = query_daemon_status(resolve_daemon_socket_path()).await?;
            println!("active_leases: {}", snapshot.active_leases);
            println!("pending_requests: {}", snapshot.pending_requests);
            for usage in snapshot.usage {
                println!(
                    "usage {} {:?} {:?}: {}/{}",
                    usage.name, usage.scope, usage.scope_key, usage.used, usage.capacity
                );
            }
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Start => {
                let socket = resolve_daemon_socket_path();
                takd::run_daemon(&socket).await?;
            }
            DaemonCommands::Status => {
                let snapshot = query_daemon_status(resolve_daemon_socket_path()).await?;
                println!("active_leases: {}", snapshot.active_leases);
                println!("pending_requests: {}", snapshot.pending_requests);
            }
        },
    }

    Ok(())
}
