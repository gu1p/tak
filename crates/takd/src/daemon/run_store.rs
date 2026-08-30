use std::path::PathBuf;

use anyhow::Result;

mod blob;
mod cancellation;
mod connection;
mod events;
mod queries;
mod scheduling;
mod schema;
mod submission;
mod types;
mod upload;

pub use types::{RunStore, SubmitRunResult, UploadProgress};

impl RunStore {
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        let blob_root = db_path.with_extension("v2-blobs");
        let store = Self { db_path, blob_root };
        store.ensure_schema()?;
        Ok(store)
    }
}
