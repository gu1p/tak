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
        unsafe { std::env::set_var(key, value.into()) }
    }

    pub fn remove(&mut self, key: &str) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
        unsafe { std::env::remove_var(key) }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.iter().rev() {
            match value {
                Some(previous) => unsafe { std::env::set_var(key, previous) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

/// Holds the global env serialization lock together with the env changes it scopes.
///
/// Field order is load-bearing: `env` is declared first so it drops (restoring the process
/// environment) BEFORE `_lock` releases. Storing a bare `MutexGuard` and `EnvGuard` separately
/// in a struct — which drops fields in declaration order — easily inverts this and restores env
/// *after* unlocking, clobbering the next env-locked test. This guard makes that impossible.
pub struct LockedEnvGuard {
    env: EnvGuard,
    _lock: MutexGuard<'static, ()>,
}

impl LockedEnvGuard {
    pub fn acquire() -> Self {
        let lock = env_lock();
        Self {
            env: EnvGuard::default(),
            _lock: lock,
        }
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.env.set(key, value);
    }

    pub fn remove(&mut self, key: &str) {
        self.env.remove(key);
    }

    /// Mutable access to the inner `EnvGuard`, for helpers that take `&mut EnvGuard`.
    pub fn env_mut(&mut self) -> &mut EnvGuard {
        &mut self.env
    }
}
