use std::path::Path;

use super::{ParsedAuthoredSource, SpecVersionMarker};

fn declaration(source: &str) -> Option<SpecVersionMarker> {
    ParsedAuthoredSource::parse(Path::new("TASKS.py"), source)
        .expect("parse source")
        .module_declaration()
        .expect("classify declaration")
        .map(|declaration| declaration.version)
}

#[test]
fn accepts_whole_top_level_bare_module_spec_calls() {
    for source in [
        "SPEC = module_spec(tasks=[])\nSPEC\n",
        "spec: ModuleSpec = module_spec(tasks=[])\nspec\n",
        "module_spec(tasks=[])\n",
        "SPEC = (module_spec(tasks=[]))\nSPEC\n",
    ] {
        assert_eq!(declaration(source), Some(SpecVersionMarker::Omitted));
    }
}

#[test]
fn ignores_nested_indirect_and_scoped_calls() {
    for source in [
        "def build():\n  return module_spec(tasks=[])\nSPEC = build()\nSPEC\n",
        "SPEC = wrapper(module_spec(tasks=[]))\nSPEC\n",
        "if True:\n  SPEC = module_spec(tasks=[])\nSPEC\n",
        "SPEC = tak.module_spec(tasks=[])\nSPEC\n",
        "FACTORY = module_spec\nSPEC = FACTORY(tasks=[])\nSPEC\n",
    ] {
        assert_eq!(declaration(source), None, "source:\n{source}");
    }
}

#[test]
fn rejects_multiple_direct_module_declarations() {
    let parsed = ParsedAuthoredSource::parse(
        Path::new("nested/TASKS.py"),
        "FIRST = module_spec(tasks=[])\nSECOND = module_spec(tasks=[])\nSECOND\n",
    )
    .expect("parse source");
    let message = parsed
        .module_declaration()
        .expect_err("multiple declarations should fail")
        .to_string();

    assert!(message.contains("nested/TASKS.py:2:"), "{message}");
    assert!(message.contains("more than one module_spec"), "{message}");
    assert!(
        message.contains("declare exactly one top-level"),
        "{message}"
    );
}

#[test]
fn policy_only_source_has_no_module_declaration() {
    assert_eq!(
        declaration("def choose(ctx):\n  return Decision.local()\n"),
        None
    );
}
