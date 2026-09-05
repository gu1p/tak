use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::daemon::path_cache::lock::CacheLock;

use super::RunStore;
mod files;
mod protection;
use files::{accessed, children, evict_path, tree_size};
use protection::{protected_paths, protected_workspaces};

#[derive(Default)]
pub(super) struct Reclaimed {
    pub(super) evicted: u64,
    pub(super) bytes: u64,
}

enum Entry {
    Workspace {
        key: String,
        path: PathBuf,
        base: PathBuf,
    },
    Paths {
        path: PathBuf,
    },
}

struct Candidate {
    accessed: u64,
    size: u64,
    entry: Entry,
}

pub(super) fn enforce_budget(store: &RunStore, budget: u64) -> Result<Reclaimed> {
    let mut connection = store.open_connection()?;
    let Some(_sweep) = CacheLock::try_acquire(&store.blob_root.join("gc.lock"))? else {
        return Ok(Reclaimed::default());
    };
    let workspace_protection = protected_workspaces(&connection)?;
    let path_protection = protected_paths(&connection)?;
    let (mut total, mut candidates) =
        workspace_candidates(store, &connection, &workspace_protection)?;
    let (path_total, mut paths) = path_candidates(store, &path_protection)?;
    total = total.saturating_add(path_total);
    candidates.append(&mut paths);
    candidates.sort_by(|left, right| {
        (left.accessed, entry_path(&left.entry)).cmp(&(right.accessed, entry_path(&right.entry)))
    });
    let mut reclaimed = Reclaimed {
        evicted: 0,
        bytes: 0,
    };
    if total <= budget {
        return Ok(reclaimed);
    }
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let workspace_protection = protected_workspaces(&transaction)?;
    let path_protection = protected_paths(&transaction)?;
    for candidate in candidates {
        if total <= budget {
            break;
        }
        let protected = match &candidate.entry {
            Entry::Workspace { key, .. } => workspace_protection.contains(key),
            Entry::Paths { path } => path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| path_protection.contains(name)),
        };
        if protected {
            continue;
        }
        evict(&transaction, candidate.entry)?;
        total = total.saturating_sub(candidate.size);
        reclaimed.evicted += 1;
        reclaimed.bytes = reclaimed.bytes.saturating_add(candidate.size);
    }
    transaction.commit()?;
    Ok(reclaimed)
}

fn workspace_candidates(
    store: &RunStore,
    connection: &Connection,
    protected: &BTreeSet<String>,
) -> Result<(u64, Vec<Candidate>)> {
    let mut statement = connection.prepare(
        "SELECT fingerprint,path,last_accessed_ms FROM workspace_blobs ORDER BY fingerprint",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut total = 0_u64;
    let mut candidates = Vec::new();
    for (fingerprint, stored_path, accessed) in rows {
        let path = PathBuf::from(stored_path);
        if path != store.blob_path(&fingerprint) || !path.is_file() {
            continue;
        }
        let base = store.blob_root.join("workspace-bases").join(&fingerprint);
        let size = fs::metadata(&path)?
            .len()
            .saturating_add(crate::daemon::workspace_layer::immutable_base_size(&base)?);
        total = total.saturating_add(size);
        if !protected.contains(&fingerprint) {
            candidates.push(Candidate {
                accessed: u64::try_from(accessed).unwrap_or(0),
                size,
                entry: Entry::Workspace {
                    key: fingerprint,
                    path,
                    base,
                },
            });
        }
    }
    Ok((total, candidates))
}

fn path_candidates(
    store: &RunStore,
    protected: &BTreeSet<String>,
) -> Result<(u64, Vec<Candidate>)> {
    let root = store.blob_root.join("path-caches");
    let mut total = 0_u64;
    let mut candidates = Vec::new();
    for path in children(&root)? {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_symlink() || !path.is_dir() || name.starts_with(".gc-") {
            continue;
        }
        let size = tree_size(&path)?;
        total = total.saturating_add(size);
        if !protected.contains(name) {
            candidates.push(Candidate {
                accessed: accessed(&path)?,
                size,
                entry: Entry::Paths { path },
            });
        }
    }
    Ok((total, candidates))
}

fn evict(transaction: &Transaction<'_>, entry: Entry) -> Result<()> {
    match entry {
        Entry::Workspace { key, path, base } => {
            crate::daemon::workspace_layer::evict_pair(&path, &base)?;
            transaction.execute("DELETE FROM workspace_blobs WHERE fingerprint=?1", [key])?;
        }
        Entry::Paths { path } => {
            evict_path(&path)?;
        }
    }
    Ok(())
}

fn entry_path(entry: &Entry) -> &Path {
    match entry {
        Entry::Workspace { path, .. } | Entry::Paths { path } => path,
    }
}
