use std::path::Path;

use super::{ParsedAuthoredSource, SpecVersionMarker};

fn version(argument: &str) -> SpecVersionMarker {
    let source = format!("SPEC = module_spec(tasks=[]{argument})\nSPEC\n");
    ParsedAuthoredSource::parse(Path::new("TASKS.py"), &source)
        .expect("parse source")
        .module_declaration()
        .expect("classify declaration")
        .expect("module declaration")
        .version
}

fn version_error(argument: &str) -> String {
    let source = format!("SPEC = module_spec(tasks=[]{argument})\nSPEC\n");
    let parsed = ParsedAuthoredSource::parse(Path::new("TASKS.py"), &source).expect("parse source");
    parsed
        .module_declaration()
        .expect_err("version should be rejected")
        .to_string()
}

#[test]
fn extracts_omitted_and_literal_module_versions() {
    assert_eq!(version(""), SpecVersionMarker::Omitted);
    assert_eq!(version(", spec_version=1"), SpecVersionMarker::Literal(1));
    assert_eq!(version(", spec_version=2"), SpecVersionMarker::Literal(2));
    assert_eq!(version(", spec_version=0x2"), SpecVersionMarker::Literal(2));
    assert_eq!(version(", spec_version=0"), SpecVersionMarker::Literal(0));
    assert_eq!(version(", spec_version=3"), SpecVersionMarker::Literal(3));
}

#[test]
fn rejects_dynamic_or_non_integer_versions() {
    for argument in [
        ", spec_version=VERSION",
        ", spec_version='2'",
        ", spec_version=True",
        ", spec_version=-1",
        ", spec_version=+2",
        ", spec_version=2.0",
        ", spec_version=999999999999999999999999999999999999",
    ] {
        let message = version_error(argument);
        assert!(
            message.contains("must be the integer literal 2 exactly"),
            "{argument}: {message}"
        );
        assert!(message.contains("TASKS.py:1:"), "{argument}: {message}");
    }
}

#[test]
fn rejects_keyword_expansion_as_an_authored_version() {
    let message = version_error(", **OPTIONS");
    assert!(
        message.contains("keyword expansion cannot establish"),
        "{message}"
    );
    assert!(
        message.contains("declare spec_version=2 explicitly"),
        "{message}"
    );
}

#[test]
fn explicit_literal_survives_unrelated_keyword_expansion() {
    assert_eq!(
        version(", spec_version=2, **EXTRA"),
        SpecVersionMarker::Literal(2)
    );
}

#[test]
fn positional_v2_marker_requires_the_authored_keyword() {
    let source = "SPEC = module_spec([], 2)\nSPEC\n";
    let parsed = ParsedAuthoredSource::parse(Path::new("TASKS.py"), source).expect("parse source");
    let message = parsed
        .module_declaration()
        .expect_err("positional v2 marker should be rejected")
        .to_string();

    assert!(
        message.contains("declare spec_version=2 as a keyword argument"),
        "{message}"
    );
}
