use std::collections::BTreeMap;

use serde_json::json;
use tak_core::v2::{
    Affinity, ContainerSource, PassEnv, RemoteSelection, Session, SessionReuse, Step, TaskRuntime,
};

use super::v2_resolved_run_support::sample_run;

#[test]
fn balanced_is_the_v2_remote_selection_default() {
    assert_eq!(RemoteSelection::default(), RemoteSelection::Balanced);
    assert_eq!(RemoteSelection::Sequential.as_str(), "sequential");
    assert_eq!(RemoteSelection::RoundRobin.as_str(), "round_robin");
}

#[test]
fn pass_env_is_validated_sorted_and_deduplicated_without_values() {
    let names = PassEnv::new(["TOKEN_2", "API_TOKEN", "TOKEN_2"]).expect("valid names");
    assert_eq!(names.as_strs(), ["API_TOKEN", "TOKEN_2"]);

    let error = PassEnv::new(["BAD-NAME"]).expect_err("invalid name");
    assert!(error.to_string().contains("BAD-NAME"));
}

#[test]
fn shared_workspace_requires_positive_parallelism_and_hard_affinity() {
    assert!(SessionReuse::shared_workspace(0).is_err());
    let reuse = SessionReuse::shared_workspace(2).expect("positive parallelism");
    assert!(Session::new("build", reuse.clone(), None).is_err());

    let hard = Affinity::require_same_node("build").expect("hard affinity");
    let session = Session::new("build", reuse, Some(hard.clone())).expect("session");
    assert_eq!(
        session.effective_affinity(None).expect("inherit"),
        Some(hard)
    );
}

#[test]
fn paths_session_requires_at_least_one_cache_selector() {
    let empty = Session::new("compiler", SessionReuse::Paths { paths: vec![] }, None);
    assert!(empty.is_err());
    let escaping = Session::new(
        "compiler",
        SessionReuse::Paths {
            paths: vec![tak_core::v2::OutputSelector::Path {
                value: "../secret".into(),
            }],
        },
        None,
    );
    assert!(escaping.is_err());
}

#[test]
fn shared_workspace_task_cannot_weaken_or_change_its_home() {
    let hard = Affinity::require_same_node("build").expect("hard affinity");
    let reuse = SessionReuse::shared_workspace(1).expect("reuse");
    let session = Session::new("build", reuse, Some(hard)).expect("session");
    let soft = Affinity::prefer_same_node("build").expect("soft affinity");
    let other = Affinity::require_same_node("other").expect("other affinity");

    assert!(session.effective_affinity(Some(&soft)).is_err());
    assert!(session.effective_affinity(Some(&other)).is_err());
}

#[test]
fn authored_step_debug_redacts_environment_values() {
    let step = Step::Cmd {
        argv: vec!["true".into()],
        cwd: None,
        env: BTreeMap::from([("TOKEN".into(), "never-debug-this".into())]),
    };

    let debug = format!("{step:?}");
    assert!(debug.contains("TOKEN"), "{debug}");
    assert!(!debug.contains("never-debug-this"), "{debug}");
}

#[test]
fn resolved_tasks_preserve_timeout_and_container_runtime() {
    let mut run = sample_run();
    run.tasks[0].timeout_s = Some(7);
    run.tasks[0].runtime = Some(TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    }));

    run.validate().unwrap();
    let encoded = serde_json::to_value(run).unwrap();
    assert_eq!(encoded["tasks"][0]["timeout_s"], 7);
    assert_eq!(
        encoded["tasks"][0]["runtime"],
        json!({"kind":"container","source":{"kind":"image","image":"alpine:3.20"}})
    );
}
