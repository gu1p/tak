use tak_proto::worker_v2::{DispatchDisposition, WorkerAttemptState};
use takd::SubmitAttemptStore;

use crate::support::v2_worker::dispatch;

#[test]
fn worker_attempt_store_fences_duplicates_generations_and_restart_ambiguity() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("agent.sqlite");
    let store = SubmitAttemptStore::with_db_path(db.clone()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&db).unwrap().permissions().mode() & 0o777, 0o600);
    }
    let first = dispatch(1, 1, "fence-1");
    assert_eq!(
        store.register_worker_v2_attempt(&first).unwrap(),
        DispatchDisposition::Accepted
    );
    assert_eq!(
        store.register_worker_v2_attempt(&first).unwrap(),
        DispatchDisposition::Duplicate
    );

    let conflicting_fence = dispatch(1, 1, "different-fence");
    assert!(
        store
            .register_worker_v2_attempt(&conflicting_fence)
            .is_err()
    );
    let second = dispatch(1, 2, "fence-2");
    assert_eq!(
        store.register_worker_v2_attempt(&second).unwrap(),
        DispatchDisposition::Accepted
    );
    assert_eq!(
        store.register_worker_v2_attempt(&first).unwrap(),
        DispatchDisposition::Stale
    );
    assert_eq!(
        store.observe_worker_v2_attempt(&first.identity, 0).unwrap().state,
        WorkerAttemptState::Missing
    );
    assert_eq!(
        store.observe_worker_v2_attempt(&second.identity, 0).unwrap().state,
        WorkerAttemptState::Running
    );

    let second_handle = SubmitAttemptStore::with_db_path(db.clone()).unwrap();
    assert_eq!(
        store.observe_worker_v2_attempt(&second.identity, 0).unwrap().state,
        WorkerAttemptState::Running
    );

    drop(store);
    drop(second_handle);
    let restarted = SubmitAttemptStore::with_db_path(db).unwrap();
    restarted.recover_worker_v2_attempts_after_restart().unwrap();
    assert_eq!(
        restarted
            .observe_worker_v2_attempt(&second.identity, 0)
            .unwrap()
            .state,
        WorkerAttemptState::Missing
    );
}
