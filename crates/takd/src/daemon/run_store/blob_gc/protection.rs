use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tak_core::v2::{ResolvedRun, SessionReuse};

pub(super) fn protected_workspaces(connection: &Connection) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT run.workspace_fingerprint FROM runs run WHERE \
         run.state NOT IN ('succeeded','failed','cancelled') OR EXISTS (SELECT 1 FROM \
         run_attempts attempt WHERE attempt.run_id=run.run_id AND attempt.released_at_ms IS NULL)",
    )?;
    Ok(statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?)
}

pub(super) fn protected_paths(connection: &Connection) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(
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
