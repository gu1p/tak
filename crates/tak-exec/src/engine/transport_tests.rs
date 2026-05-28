#![cfg(test)]

use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use super::StrictRemoteTarget;
use super::remote_models::StrictRemoteTransportKind;
use super::transport::{broker_socket_path, phase_timeout, preflight_timeout};
use super::transport_tor::tor_connect_timeout;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> MutexGuard<'static, ()> {
    match ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Default)]
struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn remove(&mut self, key: &str) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
        unsafe { std::env::remove_var(key) };
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

#[test]
fn direct_preflight_timeout_stays_at_one_second() {
    let target = target(StrictRemoteTransportKind::Direct);

    assert_eq!(preflight_timeout(&target), Duration::from_secs(1));
}

#[test]
fn tor_preflight_timeout_uses_tor_connect_timeout() {
    let target = target(StrictRemoteTransportKind::Tor);

    assert_eq!(preflight_timeout(&target), tor_connect_timeout());
}

#[test]
fn direct_phase_timeout_keeps_requested_value() {
    let target = target(StrictRemoteTransportKind::Direct);
    let requested = Duration::from_millis(250);

    assert_eq!(phase_timeout(&target, requested), requested);
}

#[test]
fn tor_phase_timeout_applies_tor_minimum() {
    let target = target(StrictRemoteTransportKind::Tor);
    let requested = Duration::from_millis(250);

    assert_eq!(phase_timeout(&target, requested), tor_connect_timeout());
}

#[test]
fn default_broker_socket_path_matches_shared_daemon_default() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_SOCKET");
    env.remove("XDG_RUNTIME_DIR");

    assert_eq!(
        broker_socket_path(),
        tak_core::runtime_paths::default_daemon_socket_path()
    );
}

fn target(transport_kind: StrictRemoteTransportKind) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:8080".into(),
        transport_kind,
        bearer_token: "secret".into(),
        runtime: None,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}
