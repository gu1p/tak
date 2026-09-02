use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::daemon::path_cache::ACCESS_MARKER;

pub(super) fn accessed(path: &Path, metadata: &fs::Metadata) -> u64 {
    fs::read_to_string(path.join(ACCESS_MARKER))
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| modified_ms(metadata))
}

pub(super) fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .and_then(|value| value.as_millis().try_into().ok())
        .unwrap_or(0)
}
