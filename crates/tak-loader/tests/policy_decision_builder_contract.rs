use std::fs;

use tak_loader::evaluate_named_policy_decision;

fn policy_error(source: &str, policy_name: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(&tasks_file, source).expect("write tasks");
    evaluate_named_policy_decision(&tasks_file, "//", policy_name)
        .expect_err("policy should fail")
        .to_string()
}

#[test]
fn rejects_remote_builder_reference_with_direct_call_guidance() {
    let message = policy_error(
        r#"def choose_remote(ctx):
  builder = Decision.remote
  return builder(pool="build", transport=Transport.TorOnionService(), reason=Reason.LOCAL_CPU_HIGH)
"#,
        "choose_remote",
    );
    assert!(
        message.contains("`Decision.remote` may only be used as a direct call"),
        "missing direct-call guidance: {message:#}"
    );
    assert!(
        message.contains("use `Decision.remote(...)`"),
        "missing migration guidance: {message:#}"
    );
}

#[test]
fn rejects_local_builder_reference_with_direct_call_guidance() {
    let message = policy_error(
        r#"def choose_local(ctx):
  builder = Decision.local
  return builder(runtime=Runtime.Host(), reason=Reason.DEFAULT_LOCAL_POLICY)
"#,
        "choose_local",
    );
    assert!(
        message.contains("`Decision.local` may only be used as a direct call"),
        "missing direct-call guidance: {message:#}"
    );
    assert!(
        message.contains("use `Decision.local(...)`"),
        "missing migration guidance: {message:#}"
    );
}

#[test]
fn rejects_bare_decision_namespace_reference_with_direct_call_guidance() {
    let message = policy_error(
        r#"def choose_remote(ctx):
  namespace = Decision
  return namespace.remote(pool="build", reason=Reason.LOCAL_CPU_HIGH)
"#,
        "choose_remote",
    );
    assert!(
        message
            .contains("`Decision` may only be used through the shipped TASKS.py namespace methods"),
        "missing direct-call restriction: {message:#}"
    );
    assert!(
        message.contains("namespace methods"),
        "missing direct-call guidance: {message:#}"
    );
}

#[test]
fn rejects_unsupported_builder_reference_with_direct_call_guidance() {
    let message = policy_error(
        r#"def choose_remote(ctx):
  builder = Decision.start
  return builder()
"#,
        "choose_remote",
    );
    assert!(
        message.contains("`Decision.start` is unsupported"),
        "missing unsupported builder detail: {message:#}"
    );
    assert!(
        message.contains("use `Decision.local(...)` or `Decision.remote(...)`"),
        "missing direct-call guidance: {message:#}"
    );
}
