use std::path::PathBuf;

use anyhow::Result;

mod attempts;
mod blob;
mod blob_gc;
mod cache_state;
mod cancellation;
mod connection;
mod events;
pub(in crate::daemon) mod execution;
mod maintenance;
pub(in crate::daemon) mod output_artifacts;
mod output_events;
mod queries;
pub(in crate::daemon) mod remote_attempts;
mod retention;
mod scheduling;
mod schema;
mod submission;
mod types;
mod upload;
mod workspace_uploads;

pub use types::{
    OutputArtifactChunk, RunAttachmentSnapshot, RunOutputManifest, RunStore,
    RunStoreMaintenanceConfig, RunStoreMaintenanceReport, SubmitRunResult, UploadProgress,
};

impl RunStore {
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        Self::with_db_path_and_maintenance(
            db_path,
            RunStoreMaintenanceConfig::default(),
            events::now_ms()?,
        )
    }

    pub fn with_db_path_and_maintenance(
        db_path: PathBuf,
        maintenance: RunStoreMaintenanceConfig,
        startup_now_ms: u64,
    ) -> Result<Self> {
        let blob_root = db_path.with_extension("v2-blobs");
        let store = Self {
            db_path,
            blob_root,
            maintenance,
        };
        store.initialize_database()?;
        store.ensure_schema()?;
        store.run_maintenance_at(startup_now_ms)?;
        Ok(store)
    }

    pub(in crate::daemon) fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }
}
