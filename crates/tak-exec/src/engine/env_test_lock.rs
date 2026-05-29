use std::sync::{Mutex, MutexGuard};

// Process-wide lock serializing tests that mutate global environment variables
// such as `TAKD_SOCKET`/`XDG_RUNTIME_DIR`. Several engine test modules touch the
// same vars; without one shared lock they race when run concurrently.
pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
