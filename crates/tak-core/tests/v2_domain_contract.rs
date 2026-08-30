use std::collections::BTreeMap;

use tak_core::v2::{Affinity, PassEnv, RemoteSelection, Session, SessionReuse, Step};

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
