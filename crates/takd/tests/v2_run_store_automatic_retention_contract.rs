use tak_proto::local_daemon::v2::RunEventKind;
use takd::{
    AttemptCompletion, AttemptOutputStream, RunStore, RunStoreMaintenanceConfig, SchedulerNode,
};

use crate::support::v2_run::{scheduler::commit, submission};

const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
const NOW_MS: u64 = 40 * DAY_MS;

#[test]
fn startup_expires_seven_day_payloads_but_keeps_thirty_day_metadata() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let config = RunStoreMaintenanceConfig::default();
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, NOW_MS).unwrap();
    let run_id = terminal_run(&store);
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE runs SET updated_at_ms=?2 WHERE run_id=?1",
            rusqlite::params![run_id, (NOW_MS - 8 * DAY_MS) as i64],
        )
        .unwrap();
    drop(store);

    let store = RunStore::with_db_path_and_maintenance(db, config, NOW_MS).unwrap();
    let run = store.get_run(&run_id).unwrap().unwrap();
    assert!(run.logs_expired && run.outputs_expired);
    assert_eq!(run.jobs.len(), 1);
    assert!(
        store
            .events_after(&run_id, 0)
            .unwrap()
            .iter()
            .all(|event| { !matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr) })
    );
}

#[test]
fn callable_maintenance_purges_metadata_only_after_thirty_days() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path_and_maintenance(
        db.clone(),
        RunStoreMaintenanceConfig::default(),
        NOW_MS,
    )
    .unwrap();
    let run_id = terminal_run(&store);
    set_age(&db, &run_id, 30);
    assert_eq!(store.run_maintenance_at(NOW_MS).unwrap().purged_runs, 0);
    assert!(store.get_run(&run_id).unwrap().is_some());
    set_age(&db, &run_id, 31);
    assert_eq!(store.run_maintenance_at(NOW_MS).unwrap().purged_runs, 1);
    assert!(store.get_run(&run_id).unwrap().is_none());
}

fn terminal_run(store: &RunStore) -> String {
    let run_id = commit(store, &submission("automatic-retention", "secret"), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    store
        .append_attempt_output(&command, "//:check", AttemptOutputStream::Stdout, b"secret")
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    run_id
}

fn set_age(db: &std::path::Path, run_id: &str, days: u64) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "UPDATE runs SET updated_at_ms=?2 WHERE run_id=?1",
            rusqlite::params![run_id, (NOW_MS - days * DAY_MS) as i64],
        )
        .unwrap();
}
