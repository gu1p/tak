use std::fs;

use tak_loader::{LoadOptions, load_workspace};

fn assert_rejected(source: &str, expected: &[&str]) {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(temp.path().join("TASKS.py"), source).expect("write TASKS.py");
    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("rejected DSL");
    let message = err.to_string();
    assert!(
        expected.iter().any(|needle| message.contains(needle)),
        "{err:#}"
    );
}

#[test]
fn rejects_removed_string_fields_and_registries() {
    let cases: &[(&str, &[&str])] = &[
        (
            r#"SPEC=module_spec(tasks=[task("bad", steps=[cmd("true")], execution_policy="local")]); SPEC"#,
            &[
                "Argument `execution_policy` does not match any known parameter",
                "task `bad` uses removed execution_policy; pass execution=policy_object",
            ],
        ),
        (
            r#"SESSION=session("bad", execution_policy="local", reuse=SessionReuse.Workspace()); SPEC=module_spec(tasks=[task("bad", execution=Execution.Session(SESSION))]); SPEC"#,
            &[
                "Argument `execution_policy` does not match any known parameter",
                "session `bad` uses removed execution_policy; pass execution=policy_object",
            ],
        ),
        (
            r#"SPEC=module_spec(tasks=[task("bad", steps=[cmd("true")], execution=Execution.Session("cargo"))]); SPEC"#,
            &["Execution.Session(...) expects a session(...) object, not a string"],
        ),
        (
            r#"SPEC=module_spec(sessions=[session(execution=Execution.Local(), reuse=SessionReuse.Workspace())], tasks=[]); SPEC"#,
            &[
                "module_spec(sessions=...) was removed",
                "does not match any known parameter of function `module_spec`",
            ],
        ),
        (
            r#"POLICY=execution_policy(placements=[Execution.Local()]); SPEC=module_spec(execution_policies=[POLICY], tasks=[]); SPEC"#,
            &[
                "module_spec(execution_policies=...) was removed",
                "does not match any known parameter of function `module_spec`",
            ],
        ),
        (
            r#"SPEC=module_spec(defaults={"execution_policy": "local"}, tasks=[task("bad", steps=[cmd("true")])]); SPEC"#,
            &[
                "Unknown key \"execution_policy\"",
                "module_spec(defaults=...) expects Defaults(...)",
            ],
        ),
    ];
    for (source, expected) in cases {
        assert_rejected(source, expected);
    }
}

#[test]
fn rejects_removed_policy_and_string_decide() {
    assert_rejected(
        r#"def choose(ctx): return Decision.local()
SPEC=module_spec(tasks=[task("bad", execution=Execution.Policy(choose))]); SPEC"#,
        &["`Execution.Policy(...)` is unsupported; use `Execution.Decide(...)`"],
    );
    assert_rejected(
        r#"SPEC=module_spec(tasks=[task("bad", execution=Execution.Decide("choose"))]); SPEC"#,
        &["Execution.Decide(...) expects a callable policy, not a string"],
    );
}
