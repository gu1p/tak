use std::sync::mpsc;
use std::time::Duration;

use tak_exec::ProcessSqliteConnection;

#[test]
fn live_connection_does_not_block_an_independent_open() {
    let temp = tempfile::tempdir().unwrap();
    let first = ProcessSqliteConnection::open(&temp.path().join("first.sqlite")).unwrap();
    let second_path = temp.path().join("second.sqlite");
    let (opened_tx, opened_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = ProcessSqliteConnection::open(&second_path).map(|_| ());
        let _ = opened_tx.send(result.map_err(|error| error.to_string()));
    });

    let opened = opened_rx.recv_timeout(Duration::from_secs(2));
    drop(first);
    worker.join().unwrap();
    assert!(
        matches!(opened, Ok(Ok(()))),
        "an existing connection held the process lifecycle gate: {opened:?}"
    );
}
