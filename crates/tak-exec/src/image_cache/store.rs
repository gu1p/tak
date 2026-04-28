#[path = "store/migration.rs"]
mod migration;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use sysinfo::{DiskRefreshKind, Disks};

use migration::ensure_schema;

#[derive(Debug, Clone)]
pub(super) struct CacheEntry {
    pub(super) engine: String,
    pub(super) cache_key: String,
    pub(super) source_kind: String,
    pub(super) image_id: String,
    pub(super) size_bytes: u64,
    pub(super) last_used_at_ms: i64,
    pub(super) last_refreshed_at_ms: i64,
    pub(super) is_current: bool,
}

#[derive(Debug, Clone)]
pub(super) struct FilesystemStatus {
    pub(super) available_bytes: u64,
    pub(super) total_bytes: u64,
    pub(super) free_floor_bytes: u64,
}

pub(super) fn open_cache_connection(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create image cache db parent {}", parent.display()))?;
    }
    let conn = Connection::open(db_path)
        .with_context(|| format!("open image cache db {}", db_path.display()))?;
    ensure_schema(&conn)?;
    Ok(conn)
}

pub(super) fn load_entries(conn: &Connection) -> Result<Vec<CacheEntry>> {
    let mut stmt = conn.prepare(
        "
        SELECT engine, cache_key, source_kind, image_id, size_bytes,
               last_used_at_ms, last_refreshed_at_ms, is_current
        FROM image_cache_entries
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        let size: i64 = row.get(4)?;
        let is_current: i64 = row.get(7)?;
        Ok(CacheEntry {
            engine: row.get(0)?,
            cache_key: row.get(1)?,
            source_kind: row.get(2)?,
            image_id: row.get(3)?,
            size_bytes: u64::try_from(size.max(0)).unwrap_or(0),
            last_used_at_ms: row.get(5)?,
            last_refreshed_at_ms: row.get(6)?,
            is_current: is_current != 0,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub(super) fn unique_image_usage_bytes(entries: &[CacheEntry]) -> u64 {
    let mut sizes = BTreeMap::<(&str, &str), u64>::new();
    for entry in entries {
        sizes
            .entry((&entry.engine, &entry.image_id))
            .and_modify(|size| *size = (*size).max(entry.size_bytes))
            .or_insert(entry.size_bytes);
    }
    sizes.values().copied().sum()
}

pub(super) fn filesystem_status(
    path: &Path,
    low_disk_min_free_percent: f64,
    low_disk_min_free_bytes: u64,
) -> Result<FilesystemStatus> {
    let path = filesystem_match_path(path)?;
    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_specifics(false, DiskRefreshKind::everything());
    let selected = disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().display().to_string().len())
        .or_else(|| disks.list().iter().next())
        .context("inspect image cache filesystem")?;
    let total_bytes = selected.total_space();
    let available_bytes = selected.available_space();
    let percent_floor = (total_bytes as f64 * (low_disk_min_free_percent / 100.0)).round() as u64;
    Ok(FilesystemStatus {
        available_bytes,
        total_bytes,
        free_floor_bytes: percent_floor.max(low_disk_min_free_bytes),
    })
}

fn filesystem_match_path(path: &Path) -> Result<std::path::PathBuf> {
    let path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(path);
    path.canonicalize().with_context(|| {
        format!(
            "canonicalize image cache filesystem path {}",
            path.display()
        )
    })
}

pub(super) fn unix_epoch_ms() -> i64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
}
