use std::path::PathBuf;

#[test]
fn every_production_sqlite_open_uses_the_process_lifecycle_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/daemon");
    let exec_root = root.join("../../../tak-exec/src");
    let gate = std::fs::read_to_string(exec_root.join("sqlite_connection.rs"))
        .expect("the execution stack must define one process-wide SQLite connection gate");
    assert!(gate.contains("static CONNECTION_LIFECYCLE_GATE"), "{gate}");
    assert!(gate.contains("pub struct ProcessSqliteConnection"), "{gate}");
    assert!(gate.contains("impl Drop for ProcessSqliteConnection"), "{gate}");

    assert_no_raw_connection_open(&root);
    assert_no_raw_connection_open(&exec_root.join("image_cache"));
}

fn assert_no_raw_connection_open(path: &std::path::Path) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            assert_no_raw_connection_open(&path);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = std::fs::read_to_string(&path).unwrap();
            let raw_open = source.lines().find(|line| {
                line.contains("Connection::open")
                    && !line.contains("ProcessSqliteConnection::open")
            });
            assert!(
                raw_open.is_none(),
                "{} bypasses the process lifecycle gate: {raw_open:?}",
                path.display(),
            );
        }
    }
}
