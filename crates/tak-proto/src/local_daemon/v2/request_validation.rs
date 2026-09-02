use std::collections::BTreeSet;

use tak_core::v2::RemoteRequirements;

use super::identifier::is_valid_identifier;

pub(super) fn valid_invite(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 16 * 1024 && !value.chars().any(char::is_control)
}

pub(super) fn valid_node_ids(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.len() <= 1024
        && values
            .iter()
            .all(|value| is_valid_identifier(value) && seen.insert(value.as_str()))
}

pub(super) fn valid_remote_path(value: &str) -> bool {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        return false;
    }
    let path = value.split('?').next().unwrap_or_default();
    matches!(path, "/v2/worker/logs" | "/v2/worker/tasks")
        || path
            .strip_prefix("/v2/worker/tasks/")
            .and_then(|value| value.strip_suffix("/events"))
            .is_some_and(|value| !value.is_empty() && !value.contains('/'))
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn valid_requirements(value: &RemoteRequirements) -> bool {
    value
        .transport
        .as_deref()
        .is_none_or(|transport| matches!(transport, "direct" | "tor"))
        && value
            .pool
            .iter()
            .chain(value.required_tags.iter())
            .chain(value.required_capabilities.iter())
            .all(|value| !value.trim().is_empty())
}
