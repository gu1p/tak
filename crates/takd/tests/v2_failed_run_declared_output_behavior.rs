use sha2::{Digest, Sha256};
use tak_core::v2::WorkspaceEntry;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{AttemptCompletion, DispatchCommand, RunStore, SchedulerNode};

use crate::support::v2_run;

const OUTPUT: &[u8] = b"kept";

#[test]
fn failed_keep_going_runs_retain_successful_independent_branch_outputs() {
    for (key, producer_first) in [("failed-output-first", true), ("failed-output-last", false)] {
        exercise_completion_order(key, producer_first);
    }
}

fn exercise_completion_order(key: &str, producer_first: bool) {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = v2_run::local_outputs::failed_keep_going_run(key, producer_first);
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 1)];
    for _ in 0..2 {
        let command = store.reserve_next(&nodes).unwrap().unwrap();
        if command.job_id == "job-0" {
            seed_output(&db, &command);
            store.complete_attempt(&command, succeeded()).unwrap();
        } else {
            store.complete_attempt(&command, failed()).unwrap();
        }
    }

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let artifacts = store.output_manifest(&run_id).unwrap().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "dist/survivor.txt");
    let chunk = store
        .output_chunk(&artifacts[0].artifact_id, 0, 1024)
        .unwrap()
        .unwrap();
    assert_eq!(chunk.bytes, OUTPUT);
    assert!(chunk.complete);
}

fn seed_output(db: &std::path::Path, command: &DispatchCommand) {
    let digest = format!("{:x}", Sha256::digest(OUTPUT));
    let entry =
        WorkspaceEntry::file("dist/survivor.txt", false, OUTPUT.len() as u64, &digest).unwrap();
    let output_root = db.with_extension("v2-blobs").join("outputs");
    std::fs::create_dir_all(&output_root).unwrap();
    std::fs::write(output_root.join(&digest), OUTPUT).unwrap();
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "INSERT INTO run_attempt_outputs (run_id,fencing_token,producer_task_id,path,artifact_id,entry_json) VALUES (?1,?2,'//:produce',?3,?4,?5)",
            rusqlite::params![
                command.run_id,
                command.fencing_token,
                entry.path,
                format!("artifact-{}", command.run_id),
                serde_json::to_string(&entry).unwrap()
            ],
        )
        .unwrap();
}

fn succeeded() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "a".repeat(64),
    }
}

fn failed() -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: "b".repeat(64),
        exit_code: Some(7),
    }
}
