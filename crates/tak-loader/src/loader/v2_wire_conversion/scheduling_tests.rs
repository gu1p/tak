use tak_core::v2::DefinitionScope;

use super::scheduling::{duration_millis, scaled_positive_millis, scope};

#[test]
fn scheduling_numbers_convert_to_exact_durable_millis() {
    assert_eq!(scaled_positive_millis(1.5, "slots").unwrap().get(), 1_500);
    assert_eq!(duration_millis(0.25, "backoff").unwrap(), 250);
    assert_eq!(duration_millis(1.001, "backoff").unwrap(), 1_001);
}

#[test]
fn authored_scopes_map_to_the_daemon_scope_model() {
    for (authored, expected) in [
        ("machine", DefinitionScope::Node),
        ("user", DefinitionScope::Submitter),
        ("project", DefinitionScope::Project),
        ("worktree", DefinitionScope::Worktree),
    ] {
        assert_eq!(scope(authored).unwrap(), expected);
    }
}

#[test]
fn invalid_scheduling_numbers_are_rejected() {
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(scaled_positive_millis(value, "slots").is_err());
    }
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(duration_millis(value, "backoff").is_err());
    }
    assert!(duration_millis((u64::MAX as f64) / 1_000.0, "backoff").is_err());
}
