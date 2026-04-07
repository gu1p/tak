#![allow(dead_code)]

use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn env_lock() -> MutexGuard<'static, ()> {
    match ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Default)]
pub struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
        // SAFETY: test processes serialize environment mutation through `ENV_LOCK`.
        unsafe {
            std::env::set_var(key, value.into());
        }
    }

    pub fn remove(&mut self, key: &str) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
        // SAFETY: test processes serialize environment mutation through `ENV_LOCK`.
        unsafe {
            std::env::remove_var(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.iter().rev() {
            match value {
                Some(previous) => {
                    // SAFETY: test processes serialize environment mutation through `ENV_LOCK`.
                    unsafe {
                        std::env::set_var(key, previous);
                    }
                }
                None => {
                    // SAFETY: test processes serialize environment mutation through `ENV_LOCK`.
                    unsafe {
                        std::env::remove_var(key);
                    }
                }
            }
        }
    }
}
