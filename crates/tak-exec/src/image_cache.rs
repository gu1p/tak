use std::path::Path;

use anyhow::Result;
use rusqlite::params;
use tak_proto::ImageCacheStatus;

use crate::ImageCacheOptions;

#[path = "image_cache/expiration.rs"]
mod expiration;
#[path = "image_cache/janitor.rs"]
mod janitor;
#[path = "image_cache/prune.rs"]
mod prune;
#[path = "image_cache/store.rs"]
mod store;

pub use janitor::run_image_cache_janitor_once;

use expiration::mutable_tag_is_expired_at;
use store::{
    filesystem_status, load_entries, open_cache_connection, unique_image_usage_bytes, unix_epoch_ms,
};

pub(crate) struct ImageCacheRecord<'a> {
    pub(crate) engine: &'a str,
    pub(crate) cache_key: &'a str,
    pub(crate) source_kind: &'a str,
    pub(crate) image_ref: &'a str,
    pub(crate) image_id: &'a str,
    pub(crate) size_bytes: u64,
    pub(crate) refreshed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageCacheEntry {
    pub(crate) source_kind: String,
    pub(crate) last_refreshed_at_ms: i64,
}

pub(crate) fn record_image_cache_entry(
    options: &ImageCacheOptions,
    record: ImageCacheRecord<'_>,
) -> Result<()> {
    let mut conn = open_cache_connection(&options.db_path)?;
    let now = unix_epoch_ms();
    let tx = conn.transaction()?;
    tx.execute(
        "
        UPDATE image_cache_entries
        SET is_current = 0
        WHERE engine = ?1 AND cache_key = ?2 AND image_id <> ?3
        ",
        params![record.engine, record.cache_key, record.image_id],
    )?;
    tx.execute(
        "
        INSERT INTO image_cache_entries (
            engine, cache_key, source_kind, image_ref, image_id, size_bytes,
            created_at_ms, last_used_at_ms, last_refreshed_at_ms, is_current
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7, 1)
        ON CONFLICT(engine, cache_key, image_id) DO UPDATE SET
            source_kind=excluded.source_kind,
            image_ref=excluded.image_ref,
            size_bytes=excluded.size_bytes,
            last_used_at_ms=excluded.last_used_at_ms,
            is_current=1,
            last_refreshed_at_ms=CASE
                WHEN ?8 THEN excluded.last_refreshed_at_ms
                ELSE image_cache_entries.last_refreshed_at_ms
            END
        ",
        params![
            record.engine,
            record.cache_key,
            record.source_kind,
            record.image_ref,
            record.image_id,
            i64::try_from(record.size_bytes).unwrap_or(i64::MAX),
            now,
            record.refreshed,
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub(crate) fn image_cache_entry(
    options: &ImageCacheOptions,
    engine: &str,
    cache_key: &str,
) -> Result<Option<ImageCacheEntry>> {
    let conn = open_cache_connection(&options.db_path)?;
    Ok(load_entries(&conn)?
        .into_iter()
        .find(|entry| entry.engine == engine && entry.cache_key == cache_key && entry.is_current)
        .map(|entry| ImageCacheEntry {
            source_kind: entry.source_kind,
            last_refreshed_at_ms: entry.last_refreshed_at_ms,
        }))
}

pub(crate) fn mutable_tag_entry_is_expired(
    options: &ImageCacheOptions,
    entry: &ImageCacheEntry,
) -> bool {
    entry.source_kind == "mutable"
        && mutable_tag_is_expired_at(
            entry.last_refreshed_at_ms,
            unix_epoch_ms(),
            options.mutable_tag_ttl_secs,
        )
}

pub fn image_cache_status(
    db_path: &Path,
    budget_bytes: u64,
    low_disk_min_free_percent: f64,
    low_disk_min_free_bytes: u64,
) -> Result<ImageCacheStatus> {
    let conn = open_cache_connection(db_path)?;
    let entries = load_entries(&conn)?;
    let used_bytes = unique_image_usage_bytes(&entries);
    let filesystem =
        filesystem_status(db_path, low_disk_min_free_percent, low_disk_min_free_bytes)?;
    Ok(ImageCacheStatus {
        used_bytes,
        budget_bytes,
        evictable_bytes: used_bytes,
        entry_count: u64::try_from(entries.len()).unwrap_or(u64::MAX),
        filesystem_available_bytes: filesystem.available_bytes,
        filesystem_total_bytes: filesystem.total_bytes,
        free_floor_bytes: filesystem.free_floor_bytes,
    })
}
