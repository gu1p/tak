use std::{path::Path, time::Duration};

pub fn begin_immediate(path: &Path) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(path).expect("open SQLite write gate");
    connection
        .busy_timeout(Duration::from_secs(45))
        .expect("configure SQLite write gate timeout");
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .expect("acquire SQLite write gate");
    connection
}
