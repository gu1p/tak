use std::time::Duration;

use anyhow::{Result, ensure};

use super::{RunStore, RunStoreMaintenanceReport, blob_gc, events, retention};

impl RunStore {
    pub fn run_maintenance(&self) -> Result<RunStoreMaintenanceReport> {
        self.run_maintenance_at(events::now_ms()?)
    }

    pub fn run_maintenance_at(&self, now_ms: u64) -> Result<RunStoreMaintenanceReport> {
        validate(self.maintenance)?;
        let expired_payloads = retention::expire_due_payloads(
            self,
            now_ms,
            duration_ms(self.maintenance.terminal_payload_retention)?,
        )?;
        let purged_runs = retention::purge_due_metadata(
            self,
            now_ms,
            duration_ms(self.maintenance.terminal_metadata_retention)?,
        )?;
        let reclaimed =
            blob_gc::enforce_budget(self, self.maintenance.workspace_path_blob_budget_bytes)?;
        Ok(RunStoreMaintenanceReport {
            expired_payloads,
            purged_runs,
            evicted_workspace_path_blobs: reclaimed.evicted,
            reclaimed_workspace_path_bytes: reclaimed.bytes,
        })
    }

    pub(crate) async fn maintain_periodically(self) {
        let interval = self.maintenance.sweep_interval;
        loop {
            tokio::time::sleep(interval).await;
            if let Err(error) = self.run_maintenance() {
                tracing::warn!("run-store maintenance sweep failed: {error:#}");
            }
        }
    }
}

fn validate(config: super::RunStoreMaintenanceConfig) -> Result<()> {
    ensure!(
        config.terminal_metadata_retention >= config.terminal_payload_retention,
        "run metadata retention must not be shorter than payload retention"
    );
    ensure!(
        !config.sweep_interval.is_zero(),
        "run-store sweep interval must be positive"
    );
    duration_ms(config.terminal_payload_retention)?;
    duration_ms(config.terminal_metadata_retention)?;
    Ok(())
}

fn duration_ms(duration: Duration) -> Result<u64> {
    duration
        .as_millis()
        .try_into()
        .map_err(|_| anyhow::anyhow!("run-store retention exceeds timestamp range"))
}
