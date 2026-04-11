use std::collections::BTreeMap;

use tak_core::{label::TaskLabel, planner::topo_sort};

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
