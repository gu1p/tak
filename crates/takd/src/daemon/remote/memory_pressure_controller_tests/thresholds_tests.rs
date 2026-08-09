use super::super::MemoryPressureSettings;
use super::super::pressure::{BYTES_PER_MB, PressureState, Thresholds, classify, thresholds};

fn settings() -> MemoryPressureSettings {
    MemoryPressureSettings::defaults()
}

const GIB: u64 = 1024 * 1024 * 1024;

#[test]
fn thresholds_keep_emergency_below_pause_below_resume() {
    for total in [4 * GIB, 8 * GIB, 32 * GIB, 256 * GIB] {
        let th = thresholds(&settings(), total);
        assert!(th.emergency < th.pause, "total={total} {th:?}");
        assert!(th.pause < th.resume, "total={total} {th:?}");
        assert!(th.resume < total, "total={total} {th:?}");
    }
}

#[test]
fn thresholds_apply_percentage_on_large_nodes() {
    // 100 GiB: 15% = 15 GiB exceeds the 2 GiB floor and is below total/2.
    let total = 100 * GIB;
    let th = thresholds(&settings(), total);
    assert_eq!(th.pause, total / 100 * 15);
}

#[test]
fn thresholds_use_floor_when_percentage_is_tiny() {
    // 8 GiB: 15% = 1.2 GiB < 2 GiB floor -> the floor wins.
    let total = 8 * GIB;
    let floor = settings().pause_floor_mb * BYTES_PER_MB;
    let th = thresholds(&settings(), total);
    assert_eq!(th.pause, floor.min(total / 2));
}

#[test]
fn classify_covers_each_band_including_dead_band() {
    let th = Thresholds {
        emergency: 100,
        pause: 200,
        resume: 400,
    };
    assert_eq!(classify(50, &th), PressureState::Emergency);
    assert_eq!(classify(150, &th), PressureState::Pause);
    assert_eq!(classify(300, &th), PressureState::Normal);
    assert_eq!(classify(500, &th), PressureState::Resume);
}
