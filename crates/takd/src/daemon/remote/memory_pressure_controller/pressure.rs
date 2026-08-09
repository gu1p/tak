use super::MemoryPressureSettings;

pub(super) const BYTES_PER_MB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PressureState {
    /// Plenty of headroom — no action; ensure no admission hold.
    Normal,
    /// Low memory — pause one newest container this tick.
    Pause,
    /// Critically low — pause aggressively and hold new admissions.
    Emergency,
    /// Recovered past the dead-band — unpause one this tick.
    Resume,
}

/// Memory thresholds in bytes, derived from settings + host total, with the
/// invariant `emergency < pause < resume`, all comfortably below `total`.
///
/// This assumes a realistic host total (physical RAM, GiB-scale); the controller
/// only runs on real hosts (gated by `memory_pressure_enabled`). For absurdly
/// small totals (a few bytes) the percentage math collapses, but `classify` then
/// yields `Normal` for every reachable `available`, so the controller is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Thresholds {
    pub(super) emergency: u64,
    pub(super) pause: u64,
    pub(super) resume: u64,
}

pub(super) fn thresholds(settings: &MemoryPressureSettings, total: u64) -> Thresholds {
    // `total / 100 * pct` avoids overflow vs `total * pct`.
    let pct = |p: u64| total / 100 * p;
    let floor = settings.pause_floor_mb.saturating_mul(BYTES_PER_MB);
    // Pause below the larger of pct% or the absolute floor, but never above half
    // of RAM (so the threshold stays achievable on tiny nodes).
    let pause = pct(settings.pause_pct).max(floor).min(total / 2);
    // Emergency strictly below pause.
    let emergency = pct(settings.emergency_pct).min(pause.saturating_sub(1));
    // Resume above pause with a dead-band, but achievable (<= 3/4 of RAM).
    let resume = pct(settings.resume_pct)
        .max(pause.saturating_add(pause / 4))
        .min(total / 4 * 3)
        .max(pause.saturating_add(1));
    Thresholds {
        emergency,
        pause,
        resume,
    }
}

pub(super) fn classify(available: u64, thresholds: &Thresholds) -> PressureState {
    if available < thresholds.emergency {
        PressureState::Emergency
    } else if available < thresholds.pause {
        PressureState::Pause
    } else if available > thresholds.resume {
        PressureState::Resume
    } else {
        PressureState::Normal
    }
}
