use super::RemoteNodeContext;

pub(crate) fn spawn_remote_orphan_watchdog(context: RemoteNodeContext) {
    let interval = context.runtime_config().remote_client_watchdog_interval();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match context.cancel_stale_active_executions() {
                Ok(stale_executions) => {
                    for stale in stale_executions {
                        tracing::warn!(
                            idempotency_key = %stale.idempotency_key,
                            stale_ms = stale.stale_ms,
                            "cancelled orphaned remote execution"
                        );
                    }
                }
                Err(error) => tracing::warn!("remote orphan watchdog sweep failed: {error:#}"),
            }
        }
    });
}
