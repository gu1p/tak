//! Non-destructive memory backpressure for remote workers.

use anyhow::Result;

use super::resource_pressure_controller::ResourcePressureSnapshot;
use super::runtime::{MemoryPressureSettings, RemoteRuntimeConfig};
use super::tak_container_usage::connect_docker_client;
use super::types::RemoteNodeContext;

mod engine;
mod policy;
mod pressure;
mod signal;

use engine::{
    list_managed_takd_containers, managed_containers, pause_container, unpause_container,
};
use policy::{ManagedContainer, TickAction, decide};
use pressure::{PressureState, classify, thresholds};
use signal::{MemorySignal, configured_memory_signal};

async fn run_memory_pressure_tick(
    runtime_config: &RemoteRuntimeConfig,
    signal: &dyn MemorySignal,
    settings: &MemoryPressureSettings,
    context: &RemoteNodeContext,
    snapshot: &mut ResourcePressureSnapshot,
) -> Result<()> {
    let (available, total) = signal.read();
    if total == 0 {
        return Ok(());
    }
    let state = classify(available, &thresholds(settings, total));
    record_pressure_before_engine_work(context, snapshot, state)?;

    let docker = connect_docker_client(runtime_config).await?;
    let managed = managed_containers(&list_managed_takd_containers(&docker).await?);
    let (paused, running): (Vec<ManagedContainer>, Vec<ManagedContainer>) =
        managed.into_iter().partition(|container| container.paused);
    let action = decide(state, &running, &paused, settings.min_running);
    let resumed_last = paused.len() <= 1 && matches!(action, TickAction::Unpause(_));
    let action_succeeded = execute_action(&docker, action).await;
    record_recovery_after_engine_work(
        context,
        snapshot,
        state,
        paused.is_empty() || (resumed_last && action_succeeded),
    )
}

fn record_pressure_before_engine_work(
    context: &RemoteNodeContext,
    snapshot: &mut ResourcePressureSnapshot,
    state: PressureState,
) -> Result<()> {
    if !matches!(state, PressureState::Emergency | PressureState::Pause) {
        return Ok(());
    }
    context.set_admission_held(true)?;
    let started_at = snapshot
        .episode_started_at_ms()
        .unwrap_or_else(super::query_helpers::unix_epoch_ms);
    *snapshot = ResourcePressureSnapshot::pressure(started_at);
    context.set_resource_pressure_snapshot(snapshot.clone())
}

fn record_recovery_after_engine_work(
    context: &RemoteNodeContext,
    snapshot: &mut ResourcePressureSnapshot,
    state: PressureState,
    all_resumed: bool,
) -> Result<()> {
    if state != PressureState::Resume {
        return context.set_resource_pressure_snapshot(snapshot.clone());
    }
    if all_resumed {
        context.set_admission_held(false)?;
        *snapshot = ResourcePressureSnapshot::healthy();
    } else {
        context.set_admission_held(true)?;
        let started_at = snapshot
            .episode_started_at_ms()
            .unwrap_or_else(super::query_helpers::unix_epoch_ms);
        *snapshot = ResourcePressureSnapshot::recovering(started_at, 0);
    }
    context.set_resource_pressure_snapshot(snapshot.clone())
}

async fn execute_action(docker: &bollard::Docker, action: TickAction) -> bool {
    match action {
        TickAction::Pause(ids) => {
            for id in ids {
                pause_container(docker, &id).await;
            }
            true
        }
        TickAction::Unpause(id) => unpause_container(docker, &id).await,
        TickAction::None => true,
    }
}

pub(crate) fn spawn_memory_pressure_controller(context: RemoteNodeContext) {
    let runtime_config = context.runtime_config();
    if tak_core::mock::mock_container_enabled() || !runtime_config.memory_pressure_enabled() {
        return;
    }
    let settings = runtime_config.memory_pressure();
    let signal = configured_memory_signal(&runtime_config);
    tokio::spawn(async move {
        let mut snapshot = ResourcePressureSnapshot::healthy();
        let mut ticker = tokio::time::interval(settings.interval);
        loop {
            ticker.tick().await;
            if let Err(error) = run_memory_pressure_tick(
                &runtime_config,
                signal.as_ref(),
                &settings,
                &context,
                &mut snapshot,
            )
            .await
            {
                tracing::warn!("memory pressure controller tick failed: {error:#}");
            }
        }
    });
}
