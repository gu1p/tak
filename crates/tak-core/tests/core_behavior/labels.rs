use tak_core::label::{TaskLabel, parse_label};

#[test]
fn parses_relative_label_using_current_package() {
    let label = parse_label(":build", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/web".to_string(),
            name: "build".to_string()
        }
    );
}

#[test]
fn parses_absolute_label() {
    let label = parse_label("//apps/api:test", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/api".to_string(),
            name: "test".to_string()
        }
    );
}

#[test]
fn parses_clean_absolute_label_without_slashes() {
    let label = parse_label("apps/api:test", "//apps/web").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//apps/api".to_string(),
            name: "test".to_string()
        }
    );
}

#[test]
fn parses_root_label_from_bare_task_name() {
    let label = parse_label("hello", "//").expect("label should parse");
    assert_eq!(
        label,
        TaskLabel {
            package: "//".to_string(),
            name: "hello".to_string()
        }
    );
}

#[test]
fn display_omits_double_slash_prefix() {
    let nested = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };
    let root = TaskLabel {
        package: "//".to_string(),
        name: "hello".to_string(),
    };

    assert_eq!(nested.to_string(), "apps/web:test");
    assert_eq!(root.to_string(), "hello");
}
