use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use tak_core::v2::{ResolvedRun, SessionReuse};

use super::RunStore;
mod files;
use files::{accessed, children, evict_path, tree_size};

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
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let protected_workspaces = protected_workspaces(&transaction)?;
    let protected_paths = protected_paths(&transaction)?;
    let (mut total, mut candidates) =
        workspace_candidates(store, &transaction, &protected_workspaces)?;
    let (path_total, mut paths) = path_candidates(store, &protected_paths)?;
    total = total.saturating_add(path_total);
    candidates.append(&mut paths);
    candidates.sort_by(|left, right| {
        (left.accessed, entry_path(&left.entry)).cmp(&(right.accessed, entry_path(&right.entry)))
    });
    let mut reclaimed = Reclaimed {
        evicted: 0,
        bytes: 0,
    };
    for candidate in candidates {
        if total <= budget {
            break;
        }
        evict(&transaction, candidate.entry)?;
        total = total.saturating_sub(candidate.size);
        reclaimed.evicted += 1;
        reclaimed.bytes = reclaimed.bytes.saturating_add(candidate.size);
    }
    transaction.commit()?;
    Ok(reclaimed)
}

fn protected_workspaces(transaction: &Transaction<'_>) -> Result<BTreeSet<String>> {
    let mut statement = transaction.prepare(
        "SELECT DISTINCT run.workspace_fingerprint FROM runs run WHERE \
         run.state NOT IN ('succeeded','failed','cancelled') OR EXISTS (SELECT 1 FROM \
         run_attempts attempt WHERE attempt.run_id=run.run_id AND attempt.released_at_ms IS NULL)",
    )?;
    Ok(statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn protected_paths(transaction: &Transaction<'_>) -> Result<BTreeSet<String>> {
    let mut statement = transaction.prepare(
        "SELECT run.run_id,run.resolved_json,attempt.job_id,attempt.node_id FROM runs run \
         JOIN run_attempts attempt ON attempt.run_id=run.run_id WHERE \
         run.state NOT IN ('succeeded','failed','cancelled') OR attempt.released_at_ms IS NULL",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut protected = BTreeSet::new();
    for (run_id, encoded, job_id, node_id) in rows {
        let run: ResolvedRun = serde_json::from_str(&encoded)?;
        let Some(session) = run
            .jobs
            .iter()
            .find(|job| job.job_id == job_id)
            .and_then(|job| job.session.as_ref())
        else {
            continue;
        };
        if matches!(session.reuse, SessionReuse::Paths { .. }) {
            let identity = serde_json::to_vec(&(&run_id, &session.id, &node_id))?;
            protected.insert(format!("{:x}", Sha256::digest(identity)));
        }
    }
    Ok(protected)
}

fn workspace_candidates(
    store: &RunStore,
    transaction: &Transaction<'_>,
    protected: &BTreeSet<String>,
) -> Result<(u64, Vec<Candidate>)> {
    let mut statement = transaction.prepare(
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
