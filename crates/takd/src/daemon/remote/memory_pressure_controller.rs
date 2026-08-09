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

use std::sync::Mutex;

use anyhow::Result;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use super::runtime::{MemoryPressureSettings, RemoteRuntimeConfig};
use super::tak_container_usage::connect_docker_client;
use super::types::RemoteNodeContext;

mod engine;
mod policy;
mod pressure;

use engine::{
    list_managed_takd_containers, managed_containers, pause_container, unpause_container,
};
use policy::{ManagedContainer, TickAction, decide};
use pressure::{PressureState, classify, thresholds};

#[path = "memory_pressure_controller_tests.rs"]
mod tests;

/// Source of the host memory reading. A trait so tests can drive pressure
/// deterministically without touching the real system.
pub(crate) trait MemorySignal: Send + Sync {
    /// `(available_bytes, total_bytes)`. `available` is MemAvailable — excludes
    /// reclaimable page cache, so it answers "can we allocate without swapping?".
    ///
    /// ```no_run
    /// # // Reason: trait method declaration with no body; private trait not reachable from a doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
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

/// One controller iteration: read pressure and take a single pause/unpause action
/// (several pauses only under Emergency). Stateless — derived from engine state.
///
/// ```no_run
/// # // Reason: async fn that connects to Docker and reads host memory; needs a tokio runtime and live engine.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
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
    // Disabled under mock and isolated test fixtures: the controller reads real
    // host memory and must never pause/hold spuriously in those environments.
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
