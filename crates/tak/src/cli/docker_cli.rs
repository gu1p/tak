use std::collections::BTreeMap;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateSpec, LocalSpec, RemoteRuntimeSpec,
    RemoteSelectionSpec, RemoteSpec, RemoteTransportKind, ResolvedTask, RetryDef, StepDef,
    TaskExecutionSpec, TaskLabel, normalize_path_ref,
};
use tak_exec::{RunOptions, TaskOutputObserver, run_resolved_task};

use super::remote_inventory::{RemoteRecord, list_remotes};
use super::remote_status::{RemoteStatusResult, fetch_remote_status_snapshot};
use super::run_output::StdStreamOutputObserver;
use super::task_history::{HistoryOutputObserver, TaskHistoryStore};

#[derive(Debug)]
pub(super) struct DockerCliSelectors {
    pub(super) local: bool,
    pub(super) node: Option<String>,
    pub(super) arch: Option<String>,
    pub(super) os: Option<String>,
    pub(super) pool: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) capabilities: Vec<String>,
    pub(super) transport: Option<String>,
}

#[derive(Debug, Default)]
struct DockerRunSpec {
    image: Option<String>,
    dockerfile: Option<String>,
    build_context: Option<String>,
    argv: Vec<String>,
    publishes: Vec<String>,
    volumes: Vec<String>,
    env: Vec<String>,
    workdir: Option<String>,
    name: Option<String>,
    cpus: Option<String>,
    memory: Option<String>,
    rm: bool,
}

pub(super) async fn run_docker_command(
    selectors: DockerCliSelectors,
    argv: Vec<String>,
) -> Result<ExitCode> {
    let Some((command, rest)) = argv.split_first() else {
        bail!("tak docker requires a Docker subcommand");
    };

    match command.as_str() {
        "build" => bail!(
            "tak docker build is not supported. Tak executes containers and does not guarantee Docker image state across invocations; use `tak docker run -f Dockerfile --build-context . ...` for a per-run Dockerfile build."
        ),
        "ps" => run_docker_ps(selectors, rest).await,
        "run" => run_docker_run(selectors, rest).await,
        other => bail!("tak docker {other} is not supported yet; supported subcommands: run, ps"),
    }
}

async fn run_docker_run(selectors: DockerCliSelectors, args: &[String]) -> Result<ExitCode> {
    let spec = parse_docker_run(args)?;
    validate_docker_run_spec(&spec)?;

    let remotes = if selectors.local {
        Vec::new()
    } else {
        matching_remotes(&selectors)?
    };
    if remotes.is_empty() && !selectors.local {
        bail!("no configured remote agents match tak docker run");
    }

    let workspace_root = std::env::current_dir().context("failed to resolve current directory")?;
    let task = docker_run_task(&selectors, &spec, remotes.first())?;
    let output_observer: Arc<dyn TaskOutputObserver> = if selectors.local {
        Arc::new(HistoryOutputObserver::new(TaskHistoryStore::open_default()?))
    } else {
        Arc::new(StdStreamOutputObserver::default())
    };
    let result = run_resolved_task(
        &task,
        &workspace_root,
        &RunOptions {
            output_observer: Some(output_observer),
            ..RunOptions::default()
        },
    )
    .await?;

    if result.success {
        return Ok(ExitCode::SUCCESS);
    }

    Ok(exit_code_from_task_result(result.exit_code))
}

async fn run_docker_ps(selectors: DockerCliSelectors, args: &[String]) -> Result<ExitCode> {
    if !args.is_empty() {
        bail!("tak docker ps does not support Docker flags yet");
    }

    let mut rows = Vec::new();
    if should_include_local_ps(&selectors) {
        rows.extend(local_ps_rows()?);
    }
    if !selectors.local {
        let remotes = matching_remotes(&selectors)?;
        let snapshot = fetch_remote_status_snapshot(&remotes).await;
        rows.extend(remote_ps_rows(&snapshot));
        for result in snapshot.iter().filter(|result| result.error.is_some()) {
            eprintln!(
                "warning: remote node {} unavailable: {}",
                result.remote.node_id,
                result.error.as_deref().unwrap_or("unknown error")
            );
        }
    }

    print!("{}", render_docker_ps(rows));
    Ok(ExitCode::SUCCESS)
}

fn parse_docker_run(args: &[String]) -> Result<DockerRunSpec> {
    let mut spec = DockerRunSpec::default();
    let mut index = 0_usize;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            spec.argv.extend(args[index + 1..].iter().cloned());
            return Ok(spec);
        }
        if arg == "-d" || arg == "--detach" {
            bail!("tak docker run does not support detached containers");
        }
        if arg == "--rm" {
            spec.rm = true;
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--publish") {
            spec.publishes.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-p" || arg == "--publish" {
            spec.publishes.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-p") {
            spec.publishes.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--volume") {
            spec.volumes.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-v" || arg == "--volume" {
            spec.volumes.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-v") {
            spec.volumes.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--env") {
            spec.env.push(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-e" || arg == "--env" {
            spec.env.push(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-e") {
            spec.env.push(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--workdir") {
            spec.workdir = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-w" || arg == "--workdir" {
            spec.workdir = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-w") {
            spec.workdir = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--name") {
            spec.name = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--name" {
            spec.name = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = long_value(arg, "--cpus") {
            spec.cpus = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--cpus" {
            spec.cpus = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = long_value(arg, "--memory") {
            spec.memory = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-m" || arg == "--memory" {
            spec.memory = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-m") {
            spec.memory = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--file") {
            spec.dockerfile = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "-f" || arg == "--file" {
            spec.dockerfile = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if let Some(value) = short_attached_value(arg, "-f") {
            spec.dockerfile = Some(value.to_string());
            index += 1;
            continue;
        }
        if let Some(value) = long_value(arg, "--build-context") {
            spec.build_context = Some(value.to_string());
            index += 1;
            continue;
        }
        if arg == "--build-context" {
            spec.build_context = Some(take_value(args, &mut index, arg)?);
            continue;
        }
        if arg.starts_with('-') {
            bail!("tak docker run does not support Docker flag `{arg}` yet");
        }

        spec.image = Some(arg.clone());
        spec.argv.extend(args[index + 1..].iter().cloned());
        return Ok(spec);
    }
    Ok(spec)
}

fn long_value<'a>(arg: &'a str, flag: &str) -> Option<&'a str> {
    arg.strip_prefix(flag)
        .and_then(|value| value.strip_prefix('='))
        .filter(|value| !value.is_empty())
}

fn short_attached_value<'a>(arg: &'a str, flag: &str) -> Option<&'a str> {
    arg.strip_prefix(flag).filter(|value| !value.is_empty())
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    let next = *index + 1;
    let Some(value) = args.get(next) else {
        bail!("{flag} requires a value");
    };
    *index += 2;
    Ok(value.clone())
}

fn validate_docker_run_spec(spec: &DockerRunSpec) -> Result<()> {
    if spec.image.is_none() && spec.dockerfile.is_none() {
        bail!("tak docker run requires an IMAGE or `-f Dockerfile`");
    }
    if spec.image.is_some() && spec.dockerfile.is_some() {
        bail!("tak docker run accepts either IMAGE or `-f Dockerfile`, not both");
    }
    if spec.argv.is_empty() {
        bail!(
            "tak docker run requires an explicit command; image default commands are not supported yet"
        );
    }
    if !spec.publishes.is_empty() {
        bail!(
            "tak docker run does not support port publishing yet; remote-to-local forwarding requires an attached tunnel"
        );
    }
    if !spec.volumes.is_empty() {
        bail!("tak docker run does not support volume mounts yet");
    }
    if spec.name.is_some() {
        bail!("tak docker run does not support --name yet");
    }
    if spec.cpus.is_some() || spec.memory.is_some() {
        bail!("tak docker run does not support resource limits yet");
    }
    let _rm_is_always_effective = spec.rm;
    Ok(())
}

fn docker_run_task(
    selectors: &DockerCliSelectors,
    spec: &DockerRunSpec,
    selected_remote: Option<&RemoteRecord>,
) -> Result<ResolvedTask> {
    let label = TaskLabel {
        package: "//".to_string(),
        name: "docker-run".to_string(),
    };
    let runtime = docker_run_runtime(spec)?;
    Ok(ResolvedTask {
        label,
        doc: "Synthetic container command created by `tak docker run`.".to_string(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: spec.argv.clone(),
            cwd: spec.workdir.clone(),
            env: parse_env(&spec.env)?,
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: Some(runtime.clone()),
        execution: docker_run_execution(selectors, runtime, selected_remote)?,
        session: None,
        cascade_execution: false,
        tags: vec!["docker".to_string(), "docker-run".to_string()],
    })
}

fn docker_run_runtime(spec: &DockerRunSpec) -> Result<RemoteRuntimeSpec> {
    let source = if let Some(image) = spec.image.as_ref() {
        ContainerRuntimeSourceSpec::Image {
            image: image.clone(),
        }
    } else {
        let dockerfile = spec
            .dockerfile
            .as_ref()
            .expect("validated dockerfile runtime");
        let build_context = spec.build_context.as_deref().unwrap_or(".");
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile: normalize_path_ref("workspace", dockerfile)
                .with_context(|| format!("invalid Dockerfile path `{dockerfile}`"))?,
            build_context: normalize_path_ref("workspace", build_context)
                .with_context(|| format!("invalid build context path `{build_context}`"))?,
        }
    };
    Ok(RemoteRuntimeSpec::Containerized { source })
}

fn docker_run_execution(
    selectors: &DockerCliSelectors,
    runtime: RemoteRuntimeSpec,
    selected_remote: Option<&RemoteRecord>,
) -> Result<TaskExecutionSpec> {
    if selectors.local {
        return Ok(TaskExecutionSpec::LocalOnly(LocalSpec {
            runtime: Some(runtime),
            ..LocalSpec::default()
        }));
    }

    Ok(TaskExecutionSpec::RemoteOnly(RemoteSpec {
        pool: selectors.pool.clone(),
        required_tags: selectors.tags.clone(),
        required_capabilities: required_remote_capabilities(selectors, selected_remote),
        transport_kind: selected_transport_kind(selectors.transport.as_deref())?,
        runtime: Some(runtime),
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    }))
}

fn required_remote_capabilities(
    selectors: &DockerCliSelectors,
    selected_remote: Option<&RemoteRecord>,
) -> Vec<String> {
    let mut capabilities = selectors.capabilities.clone();
    if let Some(arch) = selectors.arch.as_deref() {
        capabilities.push(format!("arch:{}", normalize_arch(arch)));
    }
    if let Some(os) = selectors.os.as_deref() {
        capabilities.push(format!("os:{}", normalize_os(os)));
    }
    if selectors.node.is_some()
        && let Some(remote) = selected_remote
    {
        capabilities.push(format!("node:{}", remote.node_id));
    }
    capabilities
}

fn selected_transport_kind(transport: Option<&str>) -> Result<RemoteTransportKind> {
    match transport {
        None | Some("any") => Ok(RemoteTransportKind::Any),
        Some("direct") => Ok(RemoteTransportKind::Direct),
        Some("tor") => Ok(RemoteTransportKind::Tor),
        Some(other) => {
            bail!("unsupported remote transport `{other}`; expected direct, tor, or any")
        }
    }
}

fn parse_env(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for entry in entries {
        let Some((key, value)) = entry.split_once('=') else {
            bail!("invalid --env value `{entry}`; expected KEY=VALUE");
        };
        if key.is_empty() {
            bail!("invalid --env value `{entry}`; key cannot be empty");
        }
        env.insert(key.to_string(), value.to_string());
    }
    Ok(env)
}

fn exit_code_from_task_result(code: Option<i32>) -> ExitCode {
    let normalized = code.unwrap_or(1).clamp(1, u8::MAX as i32) as u8;
    ExitCode::from(normalized)
}

#[derive(Debug)]
struct DockerPsRow {
    node: String,
    kind: String,
    task_label: String,
    task_run_id: String,
    attempt: u32,
    started_at_ms: i64,
    runtime: String,
    source: String,
    command: String,
}

fn local_ps_rows() -> Result<Vec<DockerPsRow>> {
    let rows = TaskHistoryStore::open_default()?.active_container_runs()?;
    Ok(rows
        .into_iter()
        .map(|row| DockerPsRow {
            node: "local".to_string(),
            kind: normalize_ps_kind(&row.origin, &row.task_label),
            task_label: row.task_label,
            task_run_id: row.task_run_id,
            attempt: row.attempts,
            started_at_ms: row.started_at_ms,
            runtime: empty_as_none(row.runtime),
            source: empty_as_none(row.runtime_source),
            command: empty_as_none(row.command),
        })
        .collect())
}

fn remote_ps_rows(snapshot: &[RemoteStatusResult]) -> Vec<DockerPsRow> {
    snapshot
        .iter()
        .filter_map(|result| {
            let node = result.remote.node_id.clone();
            result.status.as_ref().map(|status| {
                status
                    .active_jobs
                    .iter()
                    .filter(|job| job.runtime.as_deref() == Some("containerized"))
                    .map(|job| DockerPsRow {
                        node: node.clone(),
                        kind: normalize_ps_kind(
                            job.origin.as_deref().unwrap_or("task"),
                            &job.task_label,
                        ),
                        task_label: job.task_label.clone(),
                        task_run_id: job.task_run_id.clone(),
                        attempt: job.attempt,
                        started_at_ms: job.started_at_ms,
                        runtime: job.runtime.clone().unwrap_or_else(|| "none".to_string()),
                        source: job
                            .runtime_source
                            .clone()
                            .unwrap_or_else(|| "none".to_string()),
                        command: job.command.clone().unwrap_or_else(|| "none".to_string()),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect()
}

fn normalize_ps_kind(origin: &str, task_label: &str) -> String {
    match origin {
        "docker-run" | "exec" | "task" => origin.to_string(),
        "" if task_label == "//:docker-run" => "docker-run".to_string(),
        "" if task_label == "//:exec" => "exec".to_string(),
        _ => "task".to_string(),
    }
}

fn empty_as_none(value: String) -> String {
    if value.is_empty() {
        "none".to_string()
    } else {
        value
    }
}

fn render_docker_ps(mut rows: Vec<DockerPsRow>) -> String {
    rows.sort_unstable_by(|left, right| {
        left.node
            .cmp(&right.node)
            .then(left.kind.cmp(&right.kind))
            .then(left.task_label.cmp(&right.task_label))
            .then(left.task_run_id.cmp(&right.task_run_id))
    });
    let mut output = String::from("Tak Containers\n");
    if rows.is_empty() {
        output.push_str("(none)\n");
        return output;
    }
    for row in rows {
        output.push_str(&format!(
            "node={} kind={} task_label={} task_run_id={} attempt={} age={} runtime={} source={} command={}\n",
            row.node,
            row.kind,
            row.task_label,
            row.task_run_id,
            row.attempt,
            age_since(row.started_at_ms),
            row.runtime,
            row.source,
            row.command,
        ));
    }
    output
}

fn should_include_local_ps(selectors: &DockerCliSelectors) -> bool {
    selectors.local
        || (selectors.node.is_none()
            && selectors.arch.is_none()
            && selectors.os.is_none()
            && selectors.pool.is_none()
            && selectors.tags.is_empty()
            && selectors.capabilities.is_empty()
            && selectors
                .transport
                .as_deref()
                .is_none_or(|transport| transport == "any"))
}

fn age_since(started_at_ms: i64) -> String {
    let delta_s = unix_epoch_ms().saturating_sub(started_at_ms).max(0) / 1000;
    if delta_s >= 3600 {
        return format!("{}h{}m", delta_s / 3600, (delta_s % 3600) / 60);
    }
    if delta_s >= 60 {
        return format!("{}m{}s", delta_s / 60, delta_s % 60);
    }
    format!("{delta_s}s")
}

fn unix_epoch_ms() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}

fn matching_remotes(selectors: &DockerCliSelectors) -> Result<Vec<RemoteRecord>> {
    let remotes = list_remotes()?
        .into_iter()
        .filter(|remote| remote.enabled)
        .filter(|remote| selector_matches_remote(selectors, remote))
        .collect::<Vec<_>>();
    Ok(remotes)
}

fn selector_matches_remote(selectors: &DockerCliSelectors, remote: &RemoteRecord) -> bool {
    if let Some(node) = selectors.node.as_deref()
        && !node_selector_matches(node, remote)
    {
        return false;
    }
    if let Some(pool) = selectors.pool.as_deref()
        && !remote.pools.iter().any(|value| value == pool)
    {
        return false;
    }
    if let Some(transport) = selectors.transport.as_deref()
        && transport != "any"
        && remote.transport != transport
    {
        return false;
    }
    if let Some(arch) = selectors.arch.as_deref()
        && !has_capability(remote, &format!("arch:{}", normalize_arch(arch)))
    {
        return false;
    }
    if let Some(os) = selectors.os.as_deref()
        && !has_capability(remote, &format!("os:{}", normalize_os(os)))
    {
        return false;
    }
    selectors
        .tags
        .iter()
        .all(|tag| remote.tags.iter().any(|value| value == tag))
        && selectors
            .capabilities
            .iter()
            .all(|capability| has_capability(remote, capability))
}

fn node_selector_matches(selector: &str, remote: &RemoteRecord) -> bool {
    let value = selector.trim();
    !value.is_empty()
        && (remote.node_id == value
            || remote.display_name == value
            || remote.node_id.starts_with(value)
            || crate::remote_alias_for_node_id(&remote.node_id) == value)
}

fn has_capability(remote: &RemoteRecord, capability: &str) -> bool {
    remote.capabilities.iter().any(|value| {
        value == capability || normalize_capability(value) == normalize_capability(capability)
    })
}

fn normalize_capability(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_arch(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "aarch64" => "arm64".to_string(),
        other => other.to_string(),
    }
}

fn normalize_os(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "darwin" => "macos".to_string(),
        other => other.to_string(),
    }
}
