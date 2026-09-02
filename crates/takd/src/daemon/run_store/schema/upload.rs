pub(super) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workspace_blobs (
    fingerprint TEXT PRIMARY KEY,
    archive_sha256 TEXT NOT NULL,
    archive_size INTEGER NOT NULL,
    path TEXT NOT NULL,
    last_accessed_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS workspace_uploads (
    fingerprint TEXT PRIMARY KEY,
    owner_run_id TEXT NOT NULL,
    archive_sha256 TEXT NOT NULL,
    archive_size INTEGER NOT NULL,
    upload_offset INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER NOT NULL
);
"#;
