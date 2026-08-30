use std::collections::BTreeMap;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn v2_includes_stop_before_child_lookup_or_evaluation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let env = BTreeMap::from([(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    )]);
    for (name, child) in [("missing", "missing-child"), ("poison", "poison-child")] {
        let workspace = temp.path().join(name);
        write_tasks(
            &workspace,
            &format!(
                "SPEC = module_spec(spec_version=2, tasks=[], includes=[path(\"{child}\")])\nSPEC\n"
            ),
        )
        .expect("write root tasks");
        if name == "poison" {
            write_tasks(&workspace.join(child), "CHILD_MUST_NOT_BE_EVALUATED\n")
                .expect("write poison child");
        }

        let output = run_tak_output(&workspace, &["run", "//:check"], &env).expect("run tak");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "stderr:\n{stderr}");
        for token in [
            "module_spec(spec_version=2) declares includes",
            "v2 include resolution is not active",
            "no child TASKS.py was evaluated",
            "no legacy include fallback was attempted",
        ] {
            assert!(stderr.contains(token), "missing `{token}`: {stderr}");
        }
        for stale in [
            "does not resolve to a TASKS.py file",
            "CHILD_MUST_NOT_BE_EVALUATED",
            "does not load or execute v2 modules",
        ] {
            assert!(!stderr.contains(stale), "unexpected `{stale}`: {stderr}");
        }
    }
}
