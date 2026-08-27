use super::resource_admission::ResourceCapacity;
use super::resource_baseline::baseline_p95;

#[test]
fn baseline_uses_the_nearest_rank_ninety_fifth_percentile() {
    let samples = (1..=20)
        .map(|value| ResourceCapacity {
            cpu_cores: value as f64,
            memory_mb: value as u64 * 100,
        })
        .collect::<Vec<_>>();

    let baseline = baseline_p95(&samples);

    assert_eq!(baseline.cpu_cores, 19.0);
    assert_eq!(baseline.memory_mb, 1900);
}

#[test]
fn an_empty_sampling_window_has_no_measured_baseline() {
    assert_eq!(
        baseline_p95(&[]),
        ResourceCapacity {
            cpu_cores: 0.0,
            memory_mb: 0,
        }
    );
}
