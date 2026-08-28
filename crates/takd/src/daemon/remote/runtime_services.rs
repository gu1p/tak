use std::time::Duration;

use tokio::task::JoinHandle;

use super::{
    RemoteNodeContext, SubmitAttemptStore, spawn_memory_pressure_controller,
    spawn_remote_cleanup_janitor, spawn_remote_orphan_watchdog, spawn_tak_container_usage_sampler,
};

pub(crate) fn spawn_remote_runtime_services(context: RemoteNodeContext, store: SubmitAttemptStore) {
    if !context.claim_remote_runtime_services() {
        return;
    }
    tokio::spawn(supervise_runtime_services(move || {
        let mut services = spawn_remote_cleanup_janitor(context.clone(), store.clone());
        services.push(spawn_remote_orphan_watchdog(context.clone()));
        services.push(spawn_tak_container_usage_sampler(context.clone()));
        if let Some(service) = spawn_memory_pressure_controller(context.clone()) {
            services.push(service);
        }
        services
    }));
}

pub(super) async fn supervise_runtime_services<F>(mut spawn_services: F)
where
    F: FnMut() -> Vec<JoinHandle<()>>,
{
    loop {
        let services = spawn_services();
        let (result, _, remaining) = futures::future::select_all(services).await;
        for service in &remaining {
            service.abort();
        }
        for service in remaining {
            let _ = service.await;
        }
        match result {
            Ok(()) => tracing::error!("remote runtime service exited; restarting services"),
            Err(error) => {
                tracing::error!("remote runtime service failed; restarting services: {error}")
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
