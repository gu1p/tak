//! Small environment-variable parsers for daemon runtime configuration.

pub(super) fn optional_trimmed_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Option<String> {
    read_env(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn duration_from_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
    default_ms: u64,
) -> u64 {
    read_env(name)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_ms)
}

pub(super) fn u64_from_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: u64,
) -> u64 {
    read_env(name)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub(super) fn usize_from_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: usize,
) -> usize {
    read_env(name)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub(super) fn bool_from_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: bool,
) -> bool {
    match read_env(name)
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("1" | "true" | "yes" | "on") => true,
        Some("0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

/// Parse a 1..=100 percentage; out-of-range or unparsable falls back to default.
///
/// ```no_run
/// # // Reason: private free function reading process environment; not reachable from a doctest.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn percent_from_env(
    read_env: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: u64,
) -> u64 {
    read_env(name)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| (1..=100).contains(value))
        .unwrap_or(default)
}
