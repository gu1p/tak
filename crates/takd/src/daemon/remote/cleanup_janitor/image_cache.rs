use anyhow::Result;

use super::*;

pub(super) async fn run_remote_image_cache_cleanup_once(context: &RemoteNodeContext) -> Result<()> {
    let Some(image_cache) = context.image_cache_config() else {
        return Ok(());
    };
    if !active_job_keys(&context.shared_status_state())?.is_empty() {
        return Ok(());
    }
    tak_runner::run_image_cache_janitor_once(&image_cache_options(image_cache)).await
}

fn image_cache_options(config: RemoteImageCacheRuntimeConfig) -> tak_runner::ImageCacheOptions {
    tak_runner::ImageCacheOptions {
        db_path: config.db_path,
        budget_bytes: config.budget_bytes,
        mutable_tag_ttl_secs: config.mutable_tag_ttl_secs,
        sweep_interval_secs: config.sweep_interval_secs,
        low_disk_min_free_percent: config.low_disk_min_free_percent,
        low_disk_min_free_bytes: config.low_disk_min_free_bytes,
    }
}
