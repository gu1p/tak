use tak_update::version::{Version, parse_version, tag_string};

#[test]
fn parses_with_and_without_v_prefix() {
    let with = parse_version("v0.1.7").unwrap();
    let without = parse_version("0.1.7").unwrap();
    assert_eq!(with, without);
    assert_eq!(
        with,
        Version {
            major: 0,
            minor: 1,
            patch: 7,
        },
    );
}

#[test]
fn orders_by_major_then_minor_then_patch() {
    assert!(parse_version("v0.1.7").unwrap() > parse_version("v0.1.0").unwrap());
    assert!(parse_version("v0.2.0").unwrap() > parse_version("v0.1.99").unwrap());
    assert!(parse_version("v1.0.0").unwrap() > parse_version("v0.99.99").unwrap());
    assert_eq!(
        parse_version("v0.1.0").unwrap(),
        parse_version("0.1.0").unwrap(),
    );
}

#[test]
fn tag_string_round_trips() {
    assert_eq!(tag_string(parse_version("0.1.7").unwrap()), "v0.1.7");
    let v = parse_version("3.4.5").unwrap();
    assert_eq!(parse_version(&tag_string(v)).unwrap(), v);
}

#[test]
fn rejects_malformed_versions() {
    for bad in [
        "", "v", "1.2", "1.2.3.4", "v1.2.x", "x.y.z", "1..3", "v 1.2.3",
    ] {
        assert!(parse_version(bad).is_err(), "expected error for {bad:?}");
    }
}
