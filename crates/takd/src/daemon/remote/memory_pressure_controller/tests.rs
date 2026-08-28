#![cfg(test)]

use super::policy::ManagedContainer;

#[path = "../memory_pressure_controller_tests/decisions_tests.rs"]
mod decisions;
#[path = "../memory_pressure_controller_tests/engine_mapping_tests.rs"]
mod engine_mapping;
#[path = "../memory_pressure_controller_tests/engine_ownership_tests.rs"]
mod engine_ownership;
#[path = "../memory_pressure_controller_tests/selection_tests.rs"]
mod selection;
#[path = "../memory_pressure_controller_tests/thresholds_tests.rs"]
mod thresholds;

fn run(id: &str, created: i64, has_timeout: bool) -> ManagedContainer {
    ManagedContainer {
        id: id.to_string(),
        created,
        has_timeout,
        paused: false,
    }
}

fn paused(id: &str, created: i64) -> ManagedContainer {
    ManagedContainer {
        id: id.to_string(),
        created,
        has_timeout: false,
        paused: true,
    }
}
