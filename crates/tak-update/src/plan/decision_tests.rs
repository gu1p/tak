#![cfg(test)]
use super::{Decision, decide, normalize_tag};
use crate::plan::UpdateOptions;
use crate::version::parse_version;

fn options(
    current: &str,
    requested_tag: Option<&'static str>,
    allow_downgrade: bool,
    check_only: bool,
) -> UpdateOptions<'static> {
    UpdateOptions {
        repo: "owner/repo",
        target: "x86_64-unknown-linux-musl",
        current: parse_version(current).expect("current version"),
        requested_tag,
        allow_downgrade,
        check_only,
        public_key: "untrusted-test-key",
    }
}

fn target(tag: &str) -> crate::version::Version {
    parse_version(tag).expect("target version")
}

#[test]
fn normalize_tag_prefixes_a_missing_v() {
    assert_eq!(normalize_tag("1.2.3"), "v1.2.3");
    assert_eq!(normalize_tag("v1.2.3"), "v1.2.3");
}

#[test]
fn equal_version_is_up_to_date() {
    let decision = decide(&options("v1.0.0", None, false, false), target("v1.0.0")).unwrap();
    assert!(matches!(decision, Decision::UpToDate));
}

#[test]
fn newer_version_installs_or_reports_available() {
    let install = decide(&options("v1.0.0", None, false, false), target("v1.1.0")).unwrap();
    assert!(matches!(install, Decision::Install));
    let available = decide(&options("v1.0.0", None, false, true), target("v1.1.0")).unwrap();
    assert!(matches!(available, Decision::Available));
}

#[test]
fn implicit_older_is_up_to_date_but_requested_older_is_refused() {
    let implicit = decide(&options("v1.0.0", None, false, false), target("v0.9.0")).unwrap();
    assert!(matches!(implicit, Decision::UpToDate));
    let requested = decide(
        &options("v1.0.0", Some("v0.9.0"), false, false),
        target("v0.9.0"),
    );
    assert!(requested.is_err());
}

#[test]
fn allowed_downgrade_installs() {
    let decision = decide(
        &options("v1.0.0", Some("v0.9.0"), true, false),
        target("v0.9.0"),
    )
    .unwrap();
    assert!(matches!(decision, Decision::Install));
}
