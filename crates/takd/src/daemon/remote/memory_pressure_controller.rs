//! Non-destructive memory backpressure for remote workers.

use anyhow::Result;

use super::resource_pressure_controller::ResourcePressureSnapshot;
use super::runtime::{MemoryPressureSettings, RemoteRuntimeConfig};
use super::tak_container_usage::connect_docker_client;
use super::types::RemoteNodeContext;

mod engine;
mod policy;
mod pressure;
#[cfg(test)]
mod restart_tests;
mod signal;
#[cfg(test)]
mod tests;

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
) -> Result<()> {
    let (available, total) = signal.read();
    if total == 0 {
        return Ok(());
    }
    let state = classify(available, &thresholds(settings, total));
    record_pressure_before_engine_work(context, state)?;

    let docker = connect_docker_client(runtime_config).await?;
    let node_id = context.node_info()?.node_id;
    let managed = managed_containers(
        &list_managed_takd_containers(&docker, &node_id).await?,
        &node_id,
    );
    let (paused, running): (Vec<ManagedContainer>, Vec<ManagedContainer>) =
        managed.into_iter().partition(|container| container.paused);
    let action = decide(state, &running, &paused, settings.min_running);
    let resumed_last = paused.len() <= 1 && matches!(action, TickAction::Unpause(_));
    let action_succeeded = execute_action(&docker, action).await;
    record_recovery_after_engine_work(
        context,
        state,
        paused.is_empty() || (resumed_last && action_succeeded),
    )
}

fn record_pressure_before_engine_work(
    context: &RemoteNodeContext,
    state: PressureState,
) -> Result<()> {
    if !matches!(state, PressureState::Emergency | PressureState::Pause) {
        return Ok(());
    }
    context.set_admission_held(true)?;
    let snapshot = context.resource_pressure_snapshot()?;
    let started_at = snapshot
        .episode_started_at_ms()
        .unwrap_or_else(super::query_helpers::unix_epoch_ms);
    context.set_resource_pressure_snapshot(ResourcePressureSnapshot::pressure(started_at))
}

fn record_recovery_after_engine_work(
    context: &RemoteNodeContext,
    state: PressureState,
    all_resumed: bool,
) -> Result<()> {
    if state != PressureState::Resume {
        return Ok(());
    }
    let snapshot = context.resource_pressure_snapshot()?;
    if all_resumed {
        context.set_admission_held(false)?;
        context.set_resource_pressure_snapshot(ResourcePressureSnapshot::healthy())
    } else {
        context.set_admission_held(true)?;
        let started_at = snapshot
            .episode_started_at_ms()
            .unwrap_or_else(super::query_helpers::unix_epoch_ms);
        context.set_resource_pressure_snapshot(ResourcePressureSnapshot::recovering(started_at, 0))
    }
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

pub(crate) fn spawn_memory_pressure_controller(
    context: RemoteNodeContext,
) -> Option<tokio::task::JoinHandle<()>> {
    let runtime_config = context.runtime_config();
    if tak_core::mock::mock_container_enabled() || !runtime_config.memory_pressure_enabled() {
        return None;
    }
    let settings = runtime_config.memory_pressure();
    let signal = configured_memory_signal(&runtime_config);
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(settings.interval);
        loop {
            ticker.tick().await;
            if let Err(error) =
                run_memory_pressure_tick(&runtime_config, signal.as_ref(), &settings, &context)
                    .await
            {
                tracing::warn!("memory pressure controller tick failed: {error:#}");
            }
        }
    }))
}
