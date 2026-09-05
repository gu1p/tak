use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::mpsc;
use std::time::Duration;

use takd::RunStore;

#[test]
fn concurrent_maintenance_skips_an_already_running_cache_scan() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let (mut barrier, first) = crate::support::maintenance_scan::pause_scan(store.clone(), &db);
    let (sender, receiver) = mpsc::channel();
    let second = std::thread::spawn(move || {
        sender.send(store.run_maintenance_at(100)).unwrap();
    });

    let skipped = receiver.recv_timeout(Duration::from_secs(2));

    barrier.write_all(b"0").unwrap();
    drop(barrier);
    first.join().unwrap().unwrap();
    if skipped.is_err() {
        let marker = db
            .with_extension("v2-blobs")
            .join("path-caches/slow/.last-accessed-ms");
        if let Ok(mut writer) = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(marker)
        {
            writer.write_all(b"0").unwrap();
        }
    }
    second.join().unwrap();
    let report = skipped
        .expect("a concurrent cache scan must return without waiting")
        .unwrap();
    assert_eq!(report.evicted_workspace_path_blobs, 0);
    assert_eq!(report.reclaimed_workspace_path_bytes, 0);
}
