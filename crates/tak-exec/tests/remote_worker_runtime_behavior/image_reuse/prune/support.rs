use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{ImageCacheOptions, RemoteWorkerExecutionSpec, image_cache_status};

use crate::support::{shell_step, worker_spec};

pub(super) fn cache_options(db_path: std::path::PathBuf) -> ImageCacheOptions {
    ImageCacheOptions {
        db_path,
        budget_bytes: 1,
        mutable_tag_ttl_secs: 86_400,
        sweep_interval_secs: 60,
        low_disk_min_free_percent: 0.0,
        low_disk_min_free_bytes: 0,
    }
}

pub(super) fn seed_cache_entry(db_path: &std::path::Path, cache_key: &str, image_ref: &str) {
    seed_cache_entry_with_image_id(db_path, cache_key, image_ref, "sha256:test-image", 1024);
}

pub(super) fn seed_cache_entry_with_image_id(
    db_path: &std::path::Path,
    cache_key: &str,
    image_ref: &str,
    image_id: &str,
    size_bytes: u64,
) {
    seed_cache_entry_for_engine(
        db_path, "docker", cache_key, image_ref, image_id, size_bytes,
    );
}

pub(super) fn seed_cache_entry_for_engine(
    db_path: &std::path::Path,
    engine: &str,
    cache_key: &str,
    image_ref: &str,
    image_id: &str,
    size_bytes: u64,
) {
    let _ = image_cache_status(db_path, 1, 0.0, 0).expect("initialize cache db");
    let conn = rusqlite::Connection::open(db_path).expect("open cache db");
    conn.execute(
        "
        INSERT INTO image_cache_entries (
            engine, cache_key, source_kind, image_ref, image_id, size_bytes,
            created_at_ms, last_used_at_ms, last_refreshed_at_ms, is_current
        ) VALUES (?1, ?2, 'mutable', ?3, ?4, ?5, 1, 1, 1, 1)
        ",
        rusqlite::params![
            engine,
            cache_key,
            image_ref,
            image_id,
            i64::try_from(size_bytes).expect("size fits sqlite"),
        ],
    )
    .expect("seed cache entry");
}

pub(super) fn mutable_image_spec(db_path: std::path::PathBuf) -> RemoteWorkerExecutionSpec {
    let mut spec = worker_spec(
        "remote_runtime_cache_record_without_inline_prune",
        vec![shell_step("true")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        "builder-a",
    );
    spec.image_cache = Some(cache_options(db_path));
    spec
}
