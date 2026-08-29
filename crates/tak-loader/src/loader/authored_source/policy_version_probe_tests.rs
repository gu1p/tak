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
fn cyclic_module_values_are_projected_before_crossing_the_vm_boundary() {
    let source = "SPEC = module_spec(tasks=[])\nCYCLE = []\nCYCLE.append(CYCLE)\nSPEC['payload'] = CYCLE\nSPEC\n";
    assert_eq!(run_probe(source), [1, 1]);
    assert_eq!(
        run_probe("CYCLE = []\nCYCLE.append(CYCLE)\nCYCLE\n"),
        [0, 0]
    );
}

#[test]
fn tuple_and_absent_final_expressions_are_not_modules() {
    assert_eq!(run_probe("SPEC = module_spec(tasks=[])\nSPEC,\n"), [0, 0]);
    assert_eq!(run_probe("def choose(ctx):\n  return 1\n"), [0, 0]);
}

#[test]
fn arbitrary_dictionary_is_not_a_module_spec() {
    assert_eq!(run_probe("{'spec_version': 2}\n"), [0, 0]);
}

#[test]
fn probe_names_and_builtins_cannot_be_shadowed_by_authored_source() {
    let source = "__tak_internal_policy_version_probe = 0\nisinstance = 0\ndict = 0\nint = 0\nbool = 0\nstr = 0\nSPEC = module_spec(tasks=[])\nSPEC\n";
    assert_eq!(run_probe(source), [1, 1]);
}

#[test]
fn unicode_normalized_identifiers_cannot_shadow_the_probe() {
    let source = "__ｔａｋ_ｉｎｔｅｒｎａｌ_ｐｏｌｉｃｙ_ｖｅｒｓｉｏｎ_ｐｒｏｂｅ = lambda value: [1, 1]\nSPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC\n";
    assert_eq!(run_probe(source), [1, 2]);
}

#[test]
fn unicode_normalized_definition_names_cannot_shadow_the_probe() {
    let source = "def __ｔａｋ_ｉｎｔｅｒｎａｌ_ｐｏｌｉｃｙ_ｖｅｒｓｉｏｎ_ｐｒｏｂｅ(value):\n  return [1, 1]\nSPEC = module_spec(tasks=[])\nSPEC['spec_version'] = 2\nSPEC\n";
    assert_eq!(run_probe(source), [1, 2]);
}

#[test]
fn non_identifier_text_does_not_consume_probe_names() {
    let source = "# __ｔａｋ_ｉｎｔｅｒｎａｌ_ｐｏｌｉｃｙ_ｖｅｒｓｉｏｎ_ｐｒｏｂｅ\nMESSAGE = '__tak_internal_policy_version_probe'\n{'spec_version': 2}\n";
    let prepared = prepare_policy_version_probe(source).expect("prepare probe");
    assert!(
        prepared
            .initializer_source
            .starts_with("def __tak_internal_policy_version_probe(")
    );
}

#[test]
fn whole_statement_wrapping_preserves_python_expression_shapes() {
    for source in [
        "SPEC = module_spec(tasks=[])\n(SPEC)\n",
        "SPEC = module_spec(tasks=[])\n(\n  SPEC\n)\n",
        "SPEC = module_spec(tasks=[]); VALUE = 0; SPEC\n",
    ] {
        assert_eq!(run_probe(source), [1, 1], "source:\n{source}");
    }
}

#[test]
fn registered_module_without_version_uses_the_v1_default() {
    let source = "SPEC = module_spec(tasks=[])\nSPEC.pop('spec_version')\nSPEC\n";
    assert_eq!(run_probe(source), [1, 1]);
}
