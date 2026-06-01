use std::fmt::Write as _;

use super::{PeerEligibility, PeerSnapshot};
use crate::daemon::peer_manager::eligibility::resource_summary::PeerResources;

pub(super) fn has_resource_requirements(requirements: &PeerEligibility) -> bool {
    requirements.cpu_cores.is_some() || requirements.memory_mb.is_some()
}

pub(super) fn capacity_diagnostic(
    peers: &[PeerSnapshot],
    requirements: &PeerEligibility,
) -> String {
    let mut message = String::new();
    let _ = writeln!(
        message,
        "No known remote worker satisfies this task's requirements."
    );
    let _ = writeln!(message);
    let _ = writeln!(message, "Task requires:");
    if let Some(cpu) = requirements.cpu_cores {
        let _ = writeln!(message, "  cpu: {cpu:.2}");
    }
    if let Some(memory) = requirements.memory_mb {
        let _ = writeln!(message, "  memory: {memory} MB");
    }
    write_largest_known_worker(&mut message, largest_known_capacity(peers));
    let _ = writeln!(message);
    let _ = writeln!(
        message,
        "This task cannot run until a larger worker joins the network or its requirements are reduced."
    );
    let _ = write!(message, "source: {}:{}", file!(), line!());
    message
}

fn write_largest_known_worker(message: &mut String, largest: PeerResources) {
    let _ = writeln!(message);
    let _ = writeln!(message, "largest known worker:");
    match largest.cpu_total {
        Some(cpu) => {
            let _ = writeln!(message, "  cpu: {cpu:.2}");
        }
        None => {
            let _ = writeln!(message, "  cpu: unknown");
        }
    }
    match largest.memory_total_mb {
        Some(memory) => {
            let _ = writeln!(message, "  memory: {memory} MB");
        }
        None => {
            let _ = writeln!(message, "  memory: unknown");
        }
    }
}

fn largest_known_capacity(peers: &[PeerSnapshot]) -> PeerResources {
    peers
        .iter()
        .filter_map(|peer| peer.resource_summary.as_deref())
        .map(PeerResources::parse)
        .fold(PeerResources::default(), |mut largest, resources| {
            largest.cpu_total = max_f64(largest.cpu_total, resources.cpu_total);
            largest.memory_total_mb = largest.memory_total_mb.max(resources.memory_total_mb);
            largest
        })
}

fn max_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}
