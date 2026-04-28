use anyhow::Result;
use bollard::Docker;
use rusqlite::params;

use crate::ImageCacheOptions;
use crate::container_engine::ContainerEngine;
use crate::container_runtime::connect_container_engine;

use super::expiration::{entry_is_expired_mutable, eviction_age};
use super::image_cache_status;
use super::prune::remove_cached_image;
use super::store::{
    filesystem_status, load_entries, open_cache_connection, unique_image_usage_bytes, unix_epoch_ms,
};

async fn prune_image_cache_for_engine(
    docker: &Docker,
    options: &ImageCacheOptions,
    engine: &str,
    protected_image_ids: &[String],
) -> Result<()> {
    let conn = open_cache_connection(&options.db_path)?;
    let mut entries = load_entries(&conn)?;
    let filesystem = filesystem_status(
        &options.db_path,
        options.low_disk_min_free_percent,
        options.low_disk_min_free_bytes,
    )?;
    let over_budget = unique_image_usage_bytes(&entries) > options.budget_bytes;
    let low_disk = filesystem.available_bytes < filesystem.free_floor_bytes;
    if !over_budget && !low_disk {
        return Ok(());
    }

    let now = unix_epoch_ms();
    entries.sort_by(|left, right| {
        let left_expired = entry_is_expired_mutable(left, now, options.mutable_tag_ttl_secs);
        let right_expired = entry_is_expired_mutable(right, now, options.mutable_tag_ttl_secs);
        left_expired
            .cmp(&right_expired)
            .reverse()
            .then_with(|| eviction_age(left, left_expired).cmp(&eviction_age(right, right_expired)))
            .then(left.cache_key.cmp(&right.cache_key))
    });

    let mut attempted_image_ids = std::collections::BTreeSet::new();
    for entry in entries.into_iter().filter(|entry| entry.engine == engine) {
        if !attempted_image_ids.insert(entry.image_id.clone()) {
            continue;
        }
        if protected_image_ids.iter().any(|id| id == &entry.image_id) {
            continue;
        }
        if !remove_cached_image(docker, &entry.image_id).await {
            continue;
        }
        conn.execute(
            "DELETE FROM image_cache_entries WHERE engine = ?1 AND image_id = ?2",
            params![entry.engine, entry.image_id],
        )?;

        let current = image_cache_status(
            &options.db_path,
            options.budget_bytes,
            options.low_disk_min_free_percent,
            options.low_disk_min_free_bytes,
        )?;
        if current.used_bytes <= options.budget_bytes
            && current.filesystem_available_bytes >= current.free_floor_bytes
        {
            break;
        }
    }
    Ok(())
}

pub async fn run_image_cache_janitor_once(options: &ImageCacheOptions) -> Result<()> {
    let conn = open_cache_connection(&options.db_path)?;
    let engines = load_entries(&conn)?
        .into_iter()
        .map(|entry| entry.engine)
        .collect::<std::collections::BTreeSet<_>>();
    for engine in engines {
        let Some(container_engine) = parse_recorded_engine(&engine) else {
            tracing::warn!("image cache janitor skipped unknown container engine {engine}");
            continue;
        };
        let client = match connect_container_engine(container_engine).await {
            Ok(client) => client,
            Err(err) => {
                tracing::warn!(
                    "image cache janitor skipped unavailable container engine {engine}: {err:#}"
                );
                continue;
            }
        };
        prune_image_cache_for_engine(&client.docker, options, &engine, &[]).await?;
    }
    Ok(())
}

fn parse_recorded_engine(engine: &str) -> Option<ContainerEngine> {
    match engine {
        "docker" => Some(ContainerEngine::Docker),
        "podman" => Some(ContainerEngine::Podman),
        _ => None,
    }
}
