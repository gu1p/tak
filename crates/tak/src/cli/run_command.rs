use std::sync::Arc;

use tak_exec::{RunOptions, run_tasks};

use super::run_output::StdStreamOutputObserver;
use super::run_overrides::{
    RunExecutionOverrideArgs, apply_run_execution_overrides, warn_redundant_remote_container_flag,
};
use super::*;

pub(super) struct RunCliArgs {
    pub(super) labels: Vec<String>,
    pub(super) jobs: usize,
    pub(super) keep_going: bool,
    pub(super) local: bool,
    pub(super) remote: bool,
    pub(super) container: bool,
    pub(super) container_image: Option<String>,
    pub(super) container_dockerfile: Option<String>,
    pub(super) container_build_context: Option<String>,
}

pub(super) async fn run_task_command(args: RunCliArgs) -> Result<()> {
    if args.labels.is_empty() {
        bail!("run requires at least one label");
    }

    let spec = load_workspace_from_cwd()?;
    let targets = args
        .labels
        .iter()
        .map(|label| parse_input_label(&spec, label, "run"))
        .collect::<Result<Vec<_>>>()?;
    if warn_redundant_remote_container_flag(args.remote, args.container) {
        eprintln!(
            "warning: --container is redundant with --remote; remote execution already implies a containerized runtime"
        );
    }
    let spec = apply_run_execution_overrides(
        &spec,
        &targets,
        RunExecutionOverrideArgs {
            local: args.local,
            remote: args.remote,
            container: args.container,
            container_image: args.container_image.as_deref(),
            container_dockerfile: args.container_dockerfile.as_deref(),
            container_build_context: args.container_build_context.as_deref(),
        },
    )?;

    let summary = run_tasks(
        &spec,
        &targets,
        &RunOptions {
            jobs: args.jobs,
            keep_going: args.keep_going,
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
            "{}: {} (attempts={}, exit_code={}, placement={}, remote_node={}, transport={}, reason={}, context_hash={}, runtime={}, runtime_engine={}, session={}, reuse={})",
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
            result.remote_runtime_engine.as_deref().unwrap_or("none"),
            result.session_name.as_deref().unwrap_or("none"),
            result.session_reuse.as_deref().unwrap_or("none")
        );
    }

    Ok(())
}
