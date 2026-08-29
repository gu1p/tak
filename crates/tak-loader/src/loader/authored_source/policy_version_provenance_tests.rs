use monty::{MontyObject, MontyRepl, NoLimitTracker, PrintWriter};

use super::{super::PRELUDE, prepare_policy_version_probe};

fn run_probe(source: &str) -> Vec<i64> {
    let prepared = prepare_policy_version_probe(source).expect("prepare probe");
    let mut repl = MontyRepl::new("TASKS.py", NoLimitTracker);
    repl.feed_run(
        &prepared.initializer_source,
        Vec::new(),
        PrintWriter::Disabled,
    )
    .expect("initialize probe");
    let module = format!(
        "{PRELUDE}\n{}\n{}",
        prepared.activation_source, prepared.runtime_source
    );
    let value = repl
        .feed_run(&module, Vec::new(), PrintWriter::Disabled)
        .expect("run probe");
    let MontyObject::List(values) = value else {
        panic!("expected bounded list probe")
    };
    values
        .into_iter()
        .map(|value| match value {
            MontyObject::Int(value) => value,
            other => panic!("expected integer probe field, got {other:?}"),
        })
        .collect()
}

#[test]
fn module_identity_survives_unrelated_dictionary_mutation() {
    let source = "SAVED = module_spec\nmodule_spec = 0\nSPEC = SAVED(tasks=[])\nSPEC['spec_version'] = 2\nSPEC['__tak_kind'] = 'other'\nSPEC\n";
    assert_eq!(run_probe(source), [1, 2]);
}

#[test]
fn module_identity_survives_every_defaulted_field_removal() {
    for field in [
        "project_id",
        "tasks",
        "limiters",
        "queues",
        "exclude",
        "includes",
        "defaults",
    ] {
        let source = format!(
            "SPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC.pop('{field}')\nOTHER = module_spec(tasks=[])\nSPEC\n"
        );
        assert_eq!(run_probe(&source), [1, 2], "field: {field}");
    }
}

#[test]
fn an_authored_tag_alone_cannot_forge_module_identity() {
    let source = "{'__tak_kind': 'module_spec', 'spec_version': 2}\n";
    assert_eq!(run_probe(source), [0, 0]);
}

#[test]
fn complete_manual_module_payload_with_metadata_keeps_the_shape_fallback() {
    let source = "{'spec_version': 2, 'project_id': None, 'tasks': [], 'limiters': [], 'queues': [], 'exclude': [], 'includes': [], 'defaults': {}, 'ignored_metadata': True}\n";
    assert_eq!(run_probe(source), [1, 2]);
}

#[test]
fn ascii_function_binding_cannot_shadow_the_probe() {
    let source = "def __tak_internal_policy_version_probe(value):\n  return [1, 1]\nSPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC\n";
    assert_eq!(run_probe(source), [1, 2]);
}

#[test]
fn non_expression_binding_identifiers_reserve_the_probe_name() {
    for source in [
        "def __tak_internal_policy_version_probe():\n  pass\n",
        "class __tak_internal_policy_version_probe:\n  pass\n",
        "import os as __tak_internal_policy_version_probe\n",
        "try:\n  pass\nexcept Exception as __tak_internal_policy_version_probe:\n  pass\n",
    ] {
        let prepared = prepare_policy_version_probe(source).expect("prepare probe");
        assert!(
            prepared
                .initializer_source
                .starts_with("def __tak_internal_policy_version_probe_1("),
            "source:\n{source}"
        );
    }
}

#[test]
fn trailing_semicolon_preserves_the_selected_expression() {
    let source = "SPEC = module_spec(tasks=[])\nSPEC;\n";
    assert_eq!(run_probe(source), [1, 1]);
}
