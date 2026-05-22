#![allow(clippy::await_holding_lock)]

mod harness;
mod support;

use harness::run_and_collect_labels;
use support::{root_label, workspace_with_dependency, workspace_with_shared_dependency};

#[tokio::test]
async fn remote_submit_payloads_identify_dependency_lineage() {
    let fmt = root_label("fmt-check");
    let check = root_label("check");
    let labels = run_and_collect_labels(|workspace| {
        let spec = workspace_with_dependency(workspace, &check, &fmt);
        (spec, vec![check.clone()])
    })
    .await;

    assert_eq!(
        labels.get("fmt-check"),
        Some(&Some("check.fmt-check".into()))
    );
    assert_eq!(labels.get("check"), Some(&Some("check".into())));
}

#[tokio::test]
async fn shared_dependencies_use_their_own_execution_label() {
    let first = root_label("lint");
    let second = root_label("test");
    let shared = root_label("setup");
    let labels = run_and_collect_labels(|workspace| {
        let spec = workspace_with_shared_dependency(workspace, &first, &second, &shared);
        (spec, vec![first.clone(), second.clone()])
    })
    .await;

    assert_eq!(labels.get("setup"), Some(&Some("setup".into())));
    assert_eq!(labels.get("lint"), Some(&Some("lint".into())));
    assert_eq!(labels.get("test"), Some(&Some("test".into())));
}

#[tokio::test]
async fn explicit_target_dependency_uses_its_own_execution_label() {
    let fmt = root_label("fmt-check");
    let check = root_label("check");
    let labels = run_and_collect_labels(|workspace| {
        let spec = workspace_with_dependency(workspace, &check, &fmt);
        (spec, vec![check.clone(), fmt.clone()])
    })
    .await;

    assert_eq!(labels.get("fmt-check"), Some(&Some("fmt-check".into())));
    assert_eq!(labels.get("check"), Some(&Some("check".into())));
}
