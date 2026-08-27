#![cfg(test)]

use super::{cpu_admission_available, host_cpu_cores_used, non_tak_cpu_cores};

#[derive(Debug, Clone, Copy)]
struct MemoryAdmissionSnapshot {
    total_bytes: u64,
    workload_envelope_bytes: u64,
    host_available_bytes: u64,
    non_tak_used_bytes: u64,
    margin_bytes: u64,
    tak_reserved_bytes: u64,
    tak_actual_bytes: u64,
    pending_startup_bytes: u64,
}

fn memory_admission_available_from_snapshot(snapshot: MemoryAdmissionSnapshot) -> u64 {
    let live_envelope = snapshot
        .total_bytes
        .saturating_sub(snapshot.non_tak_used_bytes)
        .saturating_sub(snapshot.margin_bytes);
    let effective_envelope = snapshot.workload_envelope_bytes.min(live_envelope);
    let running_claim = snapshot.tak_reserved_bytes.max(snapshot.tak_actual_bytes);
    effective_envelope
        .saturating_sub(running_claim)
        .saturating_sub(snapshot.pending_startup_bytes)
        .min(snapshot.host_available_bytes)
}

#[test]
fn cpu_available_accounts_for_non_tak_usage_and_reservations() {
    let host_used = host_cpu_cores_used(75.0, 8);
    let non_tak_used = non_tak_cpu_cores(host_used, 2.0);

    let available = cpu_admission_available(8, non_tak_used, 1.0);

    assert!((available - 3.0).abs() < 0.001);
}

#[test]
fn memory_admission_accounts_for_non_tak_use_and_host_availability() {
    let mib = 1024 * 1024;

    let available = memory_admission_available_from_snapshot(MemoryAdmissionSnapshot {
        total_bytes: 16 * 1024 * mib,
        workload_envelope_bytes: 12 * 1024 * mib,
        host_available_bytes: 4 * 1024 * mib,
        non_tak_used_bytes: 6 * 1024 * mib,
        margin_bytes: 1024 * mib,
        tak_reserved_bytes: 2 * 1024 * mib,
        tak_actual_bytes: 1024 * mib,
        pending_startup_bytes: 1024 * mib,
    });

    assert_eq!(available, 4 * 1024 * mib);
}

#[test]
fn actual_tak_usage_overrides_lower_reservation_totals() {
    let mib = 1024 * 1024;

    let available = memory_admission_available_from_snapshot(MemoryAdmissionSnapshot {
        total_bytes: 16 * 1024 * mib,
        workload_envelope_bytes: 12 * 1024 * mib,
        host_available_bytes: 6 * 1024 * mib,
        non_tak_used_bytes: 2 * 1024 * mib,
        margin_bytes: 1024 * mib,
        tak_reserved_bytes: 2 * 1024 * mib,
        tak_actual_bytes: 9 * 1024 * mib,
        pending_startup_bytes: 1024 * mib,
    });

    assert_eq!(available, 2 * 1024 * mib);
}
