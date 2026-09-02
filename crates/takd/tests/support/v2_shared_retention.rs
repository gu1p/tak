use sha2::{Digest, Sha256};
use tak_core::v2::{Affinity, Session, SessionReuse};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use super::v2_run::{scheduler::commit, submission};

pub fn active(
    store: &RunStore,
    blob_root: &std::path::Path,
    key: &str,
) -> (String, std::path::PathBuf) {
    let run_id = commit(store, &request(key), "alice");
    let root = root(blob_root, &run_id);
    (run_id, root)
}

pub fn terminal(
    store: &RunStore,
    blob_root: &std::path::Path,
    key: &str,
) -> (String, std::path::PathBuf) {
    let (run_id, root) = active(store, blob_root, key);
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    (run_id, root)
}

pub fn seed(root: &std::path::Path) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("value"), b"shared").unwrap();
}

pub fn age(db: &std::path::Path, run_id: &str, updated_at_ms: u64) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "UPDATE runs SET updated_at_ms=?2 WHERE run_id=?1",
            rusqlite::params![run_id, updated_at_ms as i64],
        )
        .unwrap();
}

fn request(key: &str) -> tak_core::v2::RunSubmission {
    let mut request = submission(key, "secret");
    let affinity = Affinity::require_same_node("shared").unwrap();
    request.run.tasks[0].affinity = Some(affinity.clone());
    request.run.jobs[0].affinity = Some(affinity.clone());
    request.run.jobs[0].session = Some(
        Session::new(
            "session-a",
            SessionReuse::shared_workspace(1).unwrap(),
            Some(affinity),
        )
        .unwrap(),
    );
    tak_core::v2::RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

fn root(blob_root: &std::path::Path, run_id: &str) -> std::path::PathBuf {
    let key = serde_json::to_vec(&(run_id, "session-a", "local")).unwrap();
    blob_root
        .join("shared-workspaces")
        .join(format!("{:x}", Sha256::digest(key)))
}
