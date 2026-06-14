//! Tunables for the never-kill memory-pressure controller.

use std::time::Duration;

use super::env_parse::{duration_from_env, percent_from_env, u64_from_env, usize_from_env};

const DEFAULT_MEMORY_PRESSURE_INTERVAL_MS: u64 = 1000;
const DEFAULT_MEMORY_PRESSURE_PAUSE_PCT: u64 = 15;
const DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB: u64 = 2048;
const DEFAULT_MEMORY_PRESSURE_RESUME_PCT: u64 = 25;
const DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT: u64 = 7;
const DEFAULT_MEMORY_PRESSURE_MIN_RUNNING: usize = 1;

/// Tunables for the never-kill memory-pressure controller. Percentages are of
/// host total memory. `pause_floor_mb` is an absolute MemAvailable floor: the
/// controller pauses when available drops below the LARGER of `pause_pct`% of
/// total or this floor, so the threshold stays sane on both large nodes (where a
/// percentage is many GiB) and small ones (where it is too tight). Invariant
/// `emergency_pct < pause_pct < resume_pct` (the resume/pause gap is the
/// hysteresis dead-band); a misconfigured set falls back to defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryPressureSettings {
    pub(crate) interval: Duration,
    pub(crate) pause_pct: u64,
    pub(crate) pause_floor_mb: u64,
    pub(crate) resume_pct: u64,
    pub(crate) emergency_pct: u64,
    pub(crate) min_running: usize,
}

impl MemoryPressureSettings {
    pub(crate) fn defaults() -> Self {
        Self {
            interval: Duration::from_millis(DEFAULT_MEMORY_PRESSURE_INTERVAL_MS),
            pause_pct: DEFAULT_MEMORY_PRESSURE_PAUSE_PCT,
            pause_floor_mb: DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB,
            resume_pct: DEFAULT_MEMORY_PRESSURE_RESUME_PCT,
            emergency_pct: DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT,
            min_running: DEFAULT_MEMORY_PRESSURE_MIN_RUNNING,
        }
    }

    pub(super) fn from_env() -> Self {
        Self {
            interval: Duration::from_millis(duration_from_env(
                "TAKD_MEMORY_PRESSURE_INTERVAL_MS",
                DEFAULT_MEMORY_PRESSURE_INTERVAL_MS,
            )),
            pause_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_PAUSE_PCT",
                DEFAULT_MEMORY_PRESSURE_PAUSE_PCT,
            ),
            pause_floor_mb: u64_from_env(
                "TAKD_MEMORY_PRESSURE_PAUSE_FLOOR_MB",
                DEFAULT_MEMORY_PRESSURE_PAUSE_FLOOR_MB,
            ),
            resume_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_RESUME_PCT",
                DEFAULT_MEMORY_PRESSURE_RESUME_PCT,
            ),
            emergency_pct: percent_from_env(
                "TAKD_MEMORY_PRESSURE_EMERGENCY_PCT",
                DEFAULT_MEMORY_PRESSURE_EMERGENCY_PCT,
            ),
            min_running: usize_from_env(
                "TAKD_MEMORY_PRESSURE_MIN_RUNNING",
                DEFAULT_MEMORY_PRESSURE_MIN_RUNNING,
            ),
        }
        .sanitized()
    }

    /// Keep the hysteresis band valid; on any ordering violation, reset the three
    /// watermarks to defaults while preserving interval/floor/min_running.
    ///
    /// ```no_run
    /// # // Reason: private method on a pub(crate) type; not reachable from a doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn sanitized(self) -> Self {
        if self.emergency_pct < self.pause_pct && self.pause_pct < self.resume_pct {
            return self;
        }
        Self {
            interval: self.interval,
            pause_floor_mb: self.pause_floor_mb,
            min_running: self.min_running,
            ..Self::defaults()
        }
    }
}
