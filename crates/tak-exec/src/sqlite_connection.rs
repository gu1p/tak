use std::ops::Deref;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

static CONNECTION_LIFECYCLE_GATE: Mutex<()> = Mutex::new(());

pub struct ProcessSqliteConnection {
    connection: Option<Connection>,
}

impl ProcessSqliteConnection {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let _gate = lock_lifecycle();
        let connection = Connection::open(path)?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    pub fn open_with_flags(path: &Path, flags: OpenFlags) -> rusqlite::Result<Self> {
        let _gate = lock_lifecycle();
        let connection = Connection::open_with_flags(path, flags)?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    pub fn transaction(&mut self) -> rusqlite::Result<Transaction<'_>> {
        self.connection.as_mut().unwrap().transaction()
    }

    pub fn transaction_with_behavior(
        &mut self,
        behavior: TransactionBehavior,
    ) -> rusqlite::Result<Transaction<'_>> {
        self.connection
            .as_mut()
            .unwrap()
            .transaction_with_behavior(behavior)
    }
}

impl Deref for ProcessSqliteConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.connection.as_ref().unwrap()
    }
}

impl Drop for ProcessSqliteConnection {
    fn drop(&mut self) {
        let _gate = lock_lifecycle();
        drop(self.connection.take());
    }
}

fn lock_lifecycle() -> MutexGuard<'static, ()> {
    CONNECTION_LIFECYCLE_GATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
