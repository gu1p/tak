use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntry;
use takd::{AttemptCompletion, DispatchCommand, RunStore};

const OUTPUT: &[u8] = b"kept";

pub fn seed(db: &std::path::Path, command: &DispatchCommand) {
    seed_value(db, command, "//:produce", "dist/survivor.txt", OUTPUT);
}

pub fn seed_value(
    db: &std::path::Path,
    command: &DispatchCommand,
    producer: &str,
    path: &str,
    contents: &[u8],
) {
    let digest = format!("{:x}", Sha256::digest(contents));
    let entry = WorkspaceEntry::file(path, false, contents.len() as u64, &digest).unwrap();
    let output_root = db.with_extension("v2-blobs").join("outputs");
    std::fs::create_dir_all(&output_root).unwrap();
    std::fs::write(output_root.join(&digest), contents).unwrap();
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "INSERT INTO run_attempt_outputs (run_id,fencing_token,producer_task_id,path,artifact_id,entry_json) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![
                command.run_id,
                command.fencing_token,
                producer,
                entry.path,
                format!("artifact-{}-{producer}", command.run_id),
                serde_json::to_string(&entry).unwrap()
            ],
        )
        .unwrap();
}

pub fn assert_survivor(store: &RunStore, run_id: &str) {
    let artifacts = store.output_manifest(run_id).unwrap().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "dist/survivor.txt");
    let chunk = store
        .output_chunk(&artifacts[0].artifact_id, 0, 1024)
        .unwrap()
        .unwrap();
    assert_eq!(chunk.bytes, OUTPUT);
    assert!(chunk.complete);
}

pub fn succeeded() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "a".repeat(64),
    }
}

pub fn failed() -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: "b".repeat(64),
        exit_code: Some(7),
    }
}
