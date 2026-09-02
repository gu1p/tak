use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RemoteImageCacheRuntimeConfig {
    pub db_path: PathBuf,
    pub budget_bytes: u64,
    pub mutable_tag_ttl_secs: u64,
    pub sweep_interval_secs: u64,
    pub low_disk_min_free_percent: f64,
    pub low_disk_min_free_bytes: u64,
}

impl RemoteImageCacheRuntimeConfig {
    pub(crate) fn runner_options(&self) -> tak_runner::ImageCacheOptions {
        tak_runner::ImageCacheOptions {
            db_path: self.db_path.clone(),
            budget_bytes: self.budget_bytes,
            mutable_tag_ttl_secs: self.mutable_tag_ttl_secs,
            sweep_interval_secs: self.sweep_interval_secs,
            low_disk_min_free_percent: self.low_disk_min_free_percent,
            low_disk_min_free_bytes: self.low_disk_min_free_bytes,
        }
    }
}
