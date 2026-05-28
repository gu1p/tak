#![allow(dead_code)]

use tak_core::model::{LimiterDef, Scope, WorkspaceSpec};
use takd::{SharedLeaseManager, new_shared_manager};

pub fn manager_for(spec: &WorkspaceSpec) -> SharedLeaseManager {
    let manager = new_shared_manager();
    {
        let mut guard = manager.lock().expect("lease manager lock");
        for (key, limiter) in &spec.limiters {
            guard.set_capacity(
                key.name.clone(),
                key.scope.clone(),
                key.scope_key.clone(),
                limiter_capacity(limiter),
            );
        }
        for (key, queue) in &spec.queues {
            guard.set_capacity(
                key.name.clone(),
                key.scope.clone(),
                key.scope_key.clone(),
                queue.slots as f64,
            );
        }
    }
    manager
}

fn limiter_capacity(limiter: &LimiterDef) -> f64 {
    match limiter {
        LimiterDef::Resource { capacity, .. } => *capacity,
        LimiterDef::Lock { .. } => 1.0,
        LimiterDef::RateLimit { burst, .. } => *burst as f64,
        LimiterDef::ProcessCap { max_running, .. } => *max_running as f64,
    }
}
