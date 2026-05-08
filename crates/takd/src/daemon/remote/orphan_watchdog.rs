use super::RemoteNodeContext;

pub(crate) fn spawn_remote_orphan_watchdog(context: RemoteNodeContext) {
    let interval = context.runtime_config().remote_client_watchdog_interval();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match context.cancel_stale_active_executions() {
                Ok(keys) => {
                    for key in keys {
                        tracing::warn!("cancelled orphaned remote execution {key}");
                    }
                }
                Err(error) => tracing::warn!("remote orphan watchdog sweep failed: {error:#}"),
            }
        }
    });
}
