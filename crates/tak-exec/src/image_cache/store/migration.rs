use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn ensure_schema(conn: &Connection) -> Result<()> {
    if image_cache_table_needs_migration(conn)? {
        migrate_image_cache_entries(conn)?;
    }
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS image_cache_entries (
            engine TEXT NOT NULL DEFAULT 'docker',
            cache_key TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            image_ref TEXT NOT NULL,
            image_id TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at_ms INTEGER NOT NULL,
            last_used_at_ms INTEGER NOT NULL,
            last_refreshed_at_ms INTEGER NOT NULL,
            is_current INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (engine, cache_key, image_id)
        );

        CREATE INDEX IF NOT EXISTS idx_image_cache_entries_last_used
        ON image_cache_entries(last_used_at_ms);

        CREATE INDEX IF NOT EXISTS idx_image_cache_entries_current
        ON image_cache_entries(engine, cache_key, is_current);
        ",
    )?;
    Ok(())
}

fn image_cache_table_needs_migration(conn: &Connection) -> Result<bool> {
    let columns = table_columns(conn)?;
    if columns.is_empty() {
        return Ok(false);
    }
    let has_new_columns = columns.contains("engine") && columns.contains("is_current");
    let mut stmt = conn.prepare("PRAGMA table_info(image_cache_entries)")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
    })?;
    let mut primary_key = BTreeMap::new();
    for row in rows {
        let (name, pk) = row?;
        if pk > 0 {
            primary_key.insert(name, pk);
        }
    }
    Ok(!has_new_columns
        || primary_key.get("engine") != Some(&1)
        || primary_key.get("cache_key") != Some(&2)
        || primary_key.get("image_id") != Some(&3))
}

fn table_columns(conn: &Connection) -> Result<BTreeSet<String>> {
    let mut stmt = conn.prepare("PRAGMA table_info(image_cache_entries)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = BTreeSet::new();
    for row in rows {
        columns.insert(row?);
    }
    Ok(columns)
}

fn migrate_image_cache_entries(conn: &Connection) -> Result<()> {
    let columns = table_columns(conn)?;
    let engine_expr = if columns.contains("engine") {
        "COALESCE(NULLIF(engine, ''), 'docker')"
    } else {
        "'docker'"
    };
    let is_current_expr = if columns.contains("is_current") {
        "is_current"
    } else {
        "1"
    };

    create_replacement_table(conn)?;
    copy_existing_rows(conn, engine_expr, is_current_expr)?;
    conn.execute_batch(
        "
        DROP TABLE image_cache_entries;
        ALTER TABLE image_cache_entries_new RENAME TO image_cache_entries;
        ",
    )?;
    Ok(())
}

fn create_replacement_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS image_cache_entries_new;
        CREATE TABLE image_cache_entries_new (
            engine TEXT NOT NULL DEFAULT 'docker',
            cache_key TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            image_ref TEXT NOT NULL,
            image_id TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at_ms INTEGER NOT NULL,
            last_used_at_ms INTEGER NOT NULL,
            last_refreshed_at_ms INTEGER NOT NULL,
            is_current INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (engine, cache_key, image_id)
        );
        ",
    )?;
    Ok(())
}

fn copy_existing_rows(conn: &Connection, engine_expr: &str, is_current_expr: &str) -> Result<()> {
    conn.execute(
        &format!(
            "
            INSERT OR REPLACE INTO image_cache_entries_new (
                engine, cache_key, source_kind, image_ref, image_id, size_bytes,
                created_at_ms, last_used_at_ms, last_refreshed_at_ms, is_current
            )
            SELECT {engine_expr}, cache_key, source_kind, image_ref, image_id, size_bytes,
                   created_at_ms, last_used_at_ms, last_refreshed_at_ms, {is_current_expr}
            FROM image_cache_entries
            "
        ),
        [],
    )?;
    Ok(())
}
