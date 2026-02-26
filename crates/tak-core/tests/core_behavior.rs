//! Behavioral tests for core label and DAG planning contracts.

use std::collections::BTreeMap;

use tak_core::{
    label::{TaskLabel, parse_label},
    planner::topo_sort,
};

/// Ensures relative labels are expanded using the current package namespace.
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

/// Ensures fully-qualified labels parse without package context dependency.
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

/// Ensures topological sorting places dependencies before dependents.
#[test]
fn topo_sort_returns_dependency_first_order() {
    let build = TaskLabel {
        package: "//apps/web".to_string(),
        name: "build".to_string(),
    };
    let test = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };

    let mut deps = BTreeMap::new();
    deps.insert(build.clone(), Vec::new());
    deps.insert(test.clone(), vec![build.clone()]);

    let sorted = topo_sort(&deps).expect("topo sort should succeed");
    assert_eq!(sorted, vec![build, test]);
}

/// Ensures cycle detection reports an error for cyclic dependency graphs.
#[test]
fn topo_sort_detects_cycle() {
    let a = TaskLabel {
        package: "//apps/web".to_string(),
        name: "a".to_string(),
    };
    let b = TaskLabel {
        package: "//apps/web".to_string(),
        name: "b".to_string(),
    };

    let mut deps = BTreeMap::new();
    deps.insert(a.clone(), vec![b.clone()]);
    deps.insert(b.clone(), vec![a.clone()]);

    let err = topo_sort(&deps).expect_err("should fail on cycle");
    assert!(err.to_string().contains("cycle"));
}
