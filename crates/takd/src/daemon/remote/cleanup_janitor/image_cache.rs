use anyhow::Result;

use super::*;

pub(super) async fn run_remote_image_cache_cleanup_once(context: &RemoteNodeContext) -> Result<()> {
    let Some(image_cache) = context.image_cache_config() else {
        return Ok(());
    };
    if context.resource_admission().has_reservations()? {
        return Ok(());
    }
    tak_runner::run_image_cache_janitor_once(&image_cache.runner_options()).await
}
