use std::fs;

use tak_core::model::{PolicyDecisionSpec, RemoteTransportKind};
use tak_loader::evaluate_named_policy_decision;

#[test]
fn evaluates_named_policy_to_remote_selector() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        r#"def choose_remote(ctx):
  return Decision.remote(Remote(pool="build", transport=TorOnionService()), reason=Reason.LOCAL_CPU_HIGH)
"#,
    )
    .expect("write tasks");

    match evaluate_named_policy_decision(&tasks_file, "choose_remote").expect("evaluate policy") {
        PolicyDecisionSpec::Remote { reason, remote } => {
            assert_eq!(reason, "LOCAL_CPU_HIGH");
            assert_eq!(remote.pool.as_deref(), Some("build"));
            assert_eq!(remote.transport_kind, RemoteTransportKind::Tor);
        }
        other => panic!("expected remote policy, got {other:?}"),
    }
}

#[test]
fn rejects_invalid_policy_identifiers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        "def choose_remote(ctx):\n  return Decision.local()\n",
    )
    .expect("write tasks");
    let err = evaluate_named_policy_decision(&tasks_file, "not-valid!").expect_err("invalid name");
    assert!(
        err.to_string()
            .contains("policy_name must be a valid identifier")
    );
}
