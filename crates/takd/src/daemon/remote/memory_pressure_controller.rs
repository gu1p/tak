//! Never-kill memory backpressure.
//!
//! Admission is intentionally tolerant (see `resource_admission`), so a node can
//! accept more work than its RAM nominally holds. This controller is the runtime
//! backstop: when host `MemAvailable` runs low it **pauses** (cgroup freezer, via
//! `docker pause`) the newest task containers instead of killing them, and
//! **unpauses** them once memory recovers. Pausing only halts a container's
//! growth — it does not reclaim its RSS — so relief comes from the still-running
//! (protected) tasks finishing and freeing their memory. The controller therefore
//! never pauses the oldest running container and always keeps at least
//! `min_running` running, guaranteeing forward progress (no livelock).
//!
//! State is read fresh from the engine each tick (running + paused containers),
//! so the controller holds no in-memory pause stack and cannot drift from
//! reality. Paused containers are protected from the cleanup janitor (see
//! `cleanup_inactive_takd_containers`), so a container frozen by a prior daemon
//! instance survives a restart and is resumed here by forced progress; normal
//! orphan cleanup still applies once it is running again.

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{Context, Result};
use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::models::ContainerSummary;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use super::runtime::{MemoryPressureSettings, RemoteRuntimeConfig};
use super::tak_container_usage::connect_docker_client;
use super::types::RemoteNodeContext;

#[path = "memory_pressure_controller_tests.rs"]
mod tests;

const BYTES_PER_MB: u64 = 1024 * 1024;

/// Source of the host memory reading. A trait so tests can drive pressure
/// deterministically without touching the real system.
pub(crate) trait MemorySignal: Send + Sync {
    /// `(available_bytes, total_bytes)`. `available` is MemAvailable — excludes
    /// reclaimable page cache, so it answers "can we allocate without swapping?".
    fn read(&self) -> (u64, u64);
}

struct SysinfoMemorySignal {
    system: Mutex<System>,
}

impl SysinfoMemorySignal {
    fn new() -> Self {
        Self {
            system: Mutex::new(System::new_with_specifics(
                RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
            )),
        }
    }
}

impl MemorySignal for SysinfoMemorySignal {
    fn read(&self) -> (u64, u64) {
        let Ok(mut system) = self.system.lock() else {
            return (0, 0);
        };
        system.refresh_memory();
        (system.available_memory(), system.total_memory())
    }
}

/// One takd container, distilled from the Docker list response.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedContainer {
    id: String,
    /// Docker `Created` unix timestamp; newest = largest.
    created: i64,
    /// Carries a nonzero `tak.timeout_s` label — pausing it would let its
    /// wall-clock timeout fail the step, so it must never be paused.
    has_timeout: bool,
    /// Engine state is `paused` (frozen) rather than `running`.
    paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressureState {
    /// Plenty of headroom — no action; ensure no admission hold.
    Normal,
    /// Low memory — pause one newest container this tick.
    Pause,
    /// Critically low — pause aggressively and hold new admissions.
    Emergency,
    /// Recovered past the dead-band — unpause one this tick.
    Resume,
}

/// Memory thresholds in bytes, derived from settings + host total, with the
/// invariant `emergency < pause < resume`, all comfortably below `total`.
///
/// This assumes a realistic host total (physical RAM, GiB-scale); the controller
/// only runs on real hosts (gated by `memory_pressure_enabled`). For absurdly
/// small totals (a few bytes) the percentage math collapses, but `classify` then
/// yields `Normal` for every reachable `available`, so the controller is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Thresholds {
    emergency: u64,
    pause: u64,
    resume: u64,
}

fn thresholds(settings: &MemoryPressureSettings, total: u64) -> Thresholds {
    // `total / 100 * pct` avoids overflow vs `total * pct`.
    let pct = |p: u64| total / 100 * p;
    let floor = settings.pause_floor_mb.saturating_mul(BYTES_PER_MB);
    // Pause below the larger of pct% or the absolute floor, but never above half
    // of RAM (so the threshold stays achievable on tiny nodes).
    let pause = pct(settings.pause_pct).max(floor).min(total / 2);
    // Emergency strictly below pause.
    let emergency = pct(settings.emergency_pct).min(pause.saturating_sub(1));
    // Resume above pause with a dead-band, but achievable (<= 3/4 of RAM).
    let resume = pct(settings.resume_pct)
        .max(pause.saturating_add(pause / 4))
        .min(total / 4 * 3)
        .max(pause.saturating_add(1));
    Thresholds {
        emergency,
        pause,
        resume,
    }
}

fn classify(available: u64, thresholds: &Thresholds) -> PressureState {
    if available < thresholds.emergency {
        PressureState::Emergency
    } else if available < thresholds.pause {
        PressureState::Pause
    } else if available > thresholds.resume {
        PressureState::Resume
    } else {
        PressureState::Normal
    }
}

/// Pick container ids to pause this tick from the currently-running set:
/// newest-first, never the oldest running container, never timeout-bearing, and
/// never enough to drop the running count below `min_running`. At most
/// `max_to_pause`.
fn select_pause_victims(
    running: &[ManagedContainer],
    min_running: usize,
    max_to_pause: usize,
) -> Vec<String> {
    if running.len() <= min_running {
        return Vec::new();
    }
    let budget = (running.len() - min_running).min(max_to_pause);
    if budget == 0 {
        return Vec::new();
    }
    // Protect the oldest running container so memory can always drain.
    let oldest_id = running
        .iter()
        .min_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)))
        .map(|c| c.id.as_str());
    let mut candidates: Vec<&ManagedContainer> = running
        .iter()
        .filter(|c| !c.has_timeout)
        .filter(|c| Some(c.id.as_str()) != oldest_id)
        .collect();
    // Newest first; id as a stable tiebreaker.
    candidates.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    candidates
        .into_iter()
        .take(budget)
        .map(|c| c.id.clone())
        .collect()
}

/// Pick the paused container to resume: newest-created first (resume the most
/// recently started task first), mirroring the newest-first pause policy.
fn select_unpause_target(paused: &[ManagedContainer]) -> Option<String> {
    paused
        .iter()
        .max_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)))
        .map(|c| c.id.clone())
}

/// What the controller should do this tick (admission-hold is handled separately,
/// before any fallible Docker work).
#[derive(Debug, Clone, PartialEq, Eq)]
enum TickAction {
    Pause(Vec<String>),
    Unpause(String),
    None,
}

/// Pure tick decision over the current engine state.
///
/// **Forced progress** takes priority in every band: if fewer than `min_running`
/// containers are running while work is paused, unpause one — even under
/// Emergency. Without this, pausing down to the protected runner and then having
/// that runner finish (while memory is still below the resume watermark, since
/// paused RSS is retained) would freeze the node forever. Forced progress
/// guarantees at least `min_running` always running, so memory can always drain.
fn decide(
    state: PressureState,
    running: &[ManagedContainer],
    paused: &[ManagedContainer],
    min_running: usize,
) -> TickAction {
    if running.len() < min_running
        && let Some(id) = select_unpause_target(paused)
    {
        return TickAction::Unpause(id);
    }
    match state {
        PressureState::Emergency => {
            match select_pause_victims(running, min_running, usize::MAX) {
                victims if victims.is_empty() => TickAction::None,
                victims => TickAction::Pause(victims),
            }
        }
        PressureState::Pause => {
            match select_pause_victims(running, min_running, 1).into_iter().next() {
                Some(id) => TickAction::Pause(vec![id]),
                None => TickAction::None,
            }
        }
        PressureState::Resume => match select_unpause_target(paused) {
            Some(id) => TickAction::Unpause(id),
            None => TickAction::None,
        },
        PressureState::Normal => TickAction::None,
    }
}

fn managed_containers(summaries: &[ContainerSummary]) -> Vec<ManagedContainer> {
    summaries
        .iter()
        .filter_map(|summary| {
            let id = summary.id.clone()?;
            let created = summary.created.unwrap_or(0);
            let has_timeout = summary
                .labels
                .as_ref()
                .and_then(|labels| labels.get("tak.timeout_s"))
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|seconds| seconds > 0);
            let paused = summary.state.as_deref() == Some("paused");
            Some(ManagedContainer {
                id,
                created,
                has_timeout,
                paused,
            })
        })
        .collect()
}

async fn list_managed_takd_containers(docker: &Docker) -> Result<Vec<ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["tak.owner=takd".to_string()]);
    // Both states matter: running containers are pause candidates; paused ones
    // are unpause candidates. (A paused container's status is `paused`, not
    // `running`, so a running-only filter would lose track of what we froze.)
    filters.insert(
        "status".to_string(),
        vec!["running".to_string(), "paused".to_string()],
    );
    docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .context("list managed takd containers")
}

async fn pause_container(docker: &Docker, id: &str) {
    match docker.pause_container(id).await {
        Ok(()) => tracing::info!(container_id = %id, "memory pressure: paused container"),
        Err(err) => tracing::warn!(container_id = %id, "memory pressure: pause failed: {err}"),
    }
}

async fn unpause_container(docker: &Docker, id: &str) {
    match docker.unpause_container(id).await {
        Ok(()) => tracing::info!(container_id = %id, "memory pressure: unpaused container"),
        // A 404 / not-paused means the container already finished or was resumed
        // elsewhere — harmless; the next tick reconciles from engine state.
        Err(err) => tracing::debug!(container_id = %id, "memory pressure: unpause: {err}"),
    }
}

/// One controller iteration: read pressure and take a single pause/unpause action
/// (several pauses only under Emergency). Stateless — derived from engine state.
async fn run_memory_pressure_tick(
    runtime_config: &RemoteRuntimeConfig,
    signal: &dyn MemorySignal,
    settings: &MemoryPressureSettings,
    set_admission_held: &(dyn Fn(bool) + Send + Sync),
) -> Result<()> {
    let (available, total) = signal.read();
    if total == 0 {
        return Ok(());
    }
    let state = classify(available, &thresholds(settings, total));
    // Apply the emergency admission hold from the memory signal alone, BEFORE any
    // fallible Docker work — so a transient engine error can't drop the hold while
    // memory is critical.
    set_admission_held(state == PressureState::Emergency);

    let docker = connect_docker_client(runtime_config).await?;
    let managed = managed_containers(&list_managed_takd_containers(&docker).await?);
    let (paused, running): (Vec<ManagedContainer>, Vec<ManagedContainer>) =
        managed.into_iter().partition(|container| container.paused);

    match decide(state, &running, &paused, settings.min_running) {
        TickAction::Pause(ids) => {
            for id in ids {
                pause_container(&docker, &id).await;
            }
        }
        TickAction::Unpause(id) => unpause_container(&docker, &id).await,
        TickAction::None => {}
    }
    Ok(())
}

pub(crate) fn spawn_memory_pressure_controller(context: RemoteNodeContext) {
    let runtime_config = context.runtime_config();
    // Disabled under mock and in tests (`for_tests` sets the flag false): the
    // controller reads real host memory and must never pause/hold spuriously.
    if tak_core::mock::mock_container_enabled() || !runtime_config.memory_pressure_enabled() {
        return;
    }
    let settings = runtime_config.memory_pressure();
    let signal = SysinfoMemorySignal::new();
    let set_admission_held = move |held: bool| {
        let _ = context.set_admission_held(held);
    };
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(settings.interval);
        loop {
            ticker.tick().await;
            if let Err(err) =
                run_memory_pressure_tick(&runtime_config, &signal, &settings, &set_admission_held)
                    .await
            {
                tracing::warn!("memory pressure controller tick failed: {err:#}");
            }
        }
    });
}
